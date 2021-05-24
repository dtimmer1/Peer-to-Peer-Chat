use chrono::Local;
use libb2b::{Bing2BingError, Client, ClientServerMessage, Server};
use std::{
    collections::VecDeque,
    io::Stdout,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tracing::{debug, error, trace};
use unicode_width::UnicodeWidthStr;

use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Terminal,
};

use tui_logger::TuiLoggerWidget;

use crate::{Cli, UiClientMessage};

pub type UiClientTxChannel = mpsc::UnboundedSender<UiClientMessage>;
pub type UiClientRxChannel = mpsc::UnboundedReceiver<UiClientMessage>;

#[derive(Debug, Clone, Copy)]
pub(crate) enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug)]
enum Event<I> {
    Input(I),
    Tick,
}

pub async fn start(args: Cli) -> Result<(), Bing2BingError> {
    tui_logger::init_logger(log::LevelFilter::Trace).unwrap();
    tui_logger::set_default_level(log::LevelFilter::Info);

    // this is really bad, but I can't figure out a better way to do it because the Tui widget
    // uses the `Log` crate and tokio uses `tracing`. There *are* a bunch of compatibility
    // layers and apis that are available for `tracing` but I have not figured out how
    // to have things as nice as they are in the simple ui being set via environment
    // variables.
    //
    // Regardless, it is more interseting to have the log level filtering happening
    // in the UI, see [tui_logger](https://docs.rs/tui-logger/0.6.3/tui_logger/index.html)
    // for a starting poitn to do cool stuff.

    tui_logger::set_level_for_target("libb2b", log::LevelFilter::Trace);
    tui_logger::set_level_for_target("libb2b::server", log::LevelFilter::Trace);
    tui_logger::set_level_for_target("libb2b::client", log::LevelFilter::Trace);
    tui_logger::set_level_for_target("libb2b::cmd", log::LevelFilter::Trace);
    tui_logger::set_level_for_target("libb2b::peer", log::LevelFilter::Trace);
    tui_logger::set_level_for_target("libb2b::connection", log::LevelFilter::Trace);
    tui_logger::set_level_for_target("libb2b::peer_map", log::LevelFilter::Trace);

    // let args = cli;

    trace!("Args: {:?}", args);

    let ip_address = args.ip_address.to_string().clone();
    let port = args.port; //.to_string().clone();

    let tracker_ip_address = args.tracker_ip_address.to_string().clone();
    let tracker_port = args.tracker_port; //.to_string().clone();

    let my_name = args.name;

    let max_connections = args.max_connections;

    let (ui_client_tx, ui_client_rx) = mpsc::unbounded_channel();

    let (client, server) = libb2b::init(&my_name, &ip_address, port).await;

    let network_client = client;

    let app = App::new();

    let moved_app = app.clone();
    std::thread::spawn(move || {
        debug!("STARTING PEER!!!!");
        start_peer(
            moved_app,
            network_client,
            server,
            tracker_ip_address,
            tracker_port,
            max_connections,
            ui_client_rx,
        )
    });

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let tick_rate = Duration::from_millis(200);

    // this is a thread that will fire ticks
    // to let us know that we need to update things.
    // It essentiallyc controls the refresh rate of how often we will
    // scan for input from the user.
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll failed!") {
                if let CEvent::Key(key) = event::read().expect("can't read events?") {
                    // tx.send(Event::Input(key)).expect("can't send events?");
                    if let Err(err) = tx.send(Event::Input(key)) {
                        error!(
                            "Had an error when trying to send to input key handler; breaking. {}",
                            err
                        );
                        break;
                    }
                }
            }

            if last_tick.elapsed() >= tick_rate && tx.send(Event::Tick).is_ok() {
                last_tick = Instant::now();
            }
        }
    });

    enable_raw_mode().expect("Can't start raw mode?!?!");

    let stdout = std::io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Arc::new(Mutex::new(Terminal::new(backend)?));
    let app_moved = app.clone();
    let terminal_moved = terminal.clone();

    tokio::spawn(async move {
        let app = app_moved;

        let terminal = terminal_moved;

        loop {
            match rx.recv().await {
                Some(Event::Input(event)) => {
                    match app.get_input_mode() {
                        InputMode::Editing => {
                            match event.code {
                                // KeyCode::Char('\n') => {
                                KeyCode::Enter => {
                                    debug!("got Enter");

                                    let to_say = app.input_string_drain();

                                    app.add_message(&to_say);

                                    // we also need to say this message
                                    debug!("calling client.say()");

                                    let msg = UiClientMessage::Say(to_say);

                                    ui_client_tx.send(msg).unwrap();
                                }
                                KeyCode::Char(c) => {
                                    trace!("got a character: {}", c);
                                    app.input_string_push(c);
                                }
                                KeyCode::Backspace => {
                                    app.input_string_pop();
                                }
                                KeyCode::Esc => {
                                    app.set_input_mode(InputMode::Normal);
                                }
                                _ => {}
                            }
                        }
                        InputMode::Normal => match event.code {
                            KeyCode::Char('e') => {
                                app.set_input_mode(InputMode::Editing);
                            }

                            KeyCode::Char('q') => {
                                debug!("we are shuttong down?!?! {}", app.is_shutdown());
                                disable_raw_mode().unwrap();
                                {
                                    let mut terminal = terminal.lock().unwrap();
                                    terminal.show_cursor().unwrap();
                                }

                                app.shutdown();
                                debug!("After calling shutdown() {}", app.is_shutdown());

                                break;
                            }
                            KeyCode::Char('h') => app.set_active_menu_item(MenuItem::Home),
                            KeyCode::Char('l') => app.set_active_menu_item(MenuItem::Logs),
                            _ => {}
                        },
                    }
                }

                Some(Event::Tick) => { /* we just do nothing here */ }
                None => {
                    break;
                }
            }
        }
    });

    loop {
        if app.is_shutdown() {
            debug!("App was shut down");
            break;
        }

        draw(&mut app.clone(), &mut terminal);
    }

    Ok(())
}

pub fn draw(app: &mut App, terminal: &mut Arc<Mutex<Terminal<CrosstermBackend<Stdout>>>>) {
    let menu_titles = vec!["Home", "Logs"];
    let mut terminal = terminal.lock().unwrap();
    terminal
        .draw(|rect| {
            let size = rect.size();

            let chunks = build_chunks(size);

            let input_mode = app.get_input_mode();

            let (msg, style) = get_input_mode_message(input_mode);

            let mut text = Text::from(Spans::from(msg));

            text.patch_style(style);

            let input_mode = app.get_input_mode();

            let input = get_input_box(app.clone_input_string(), input_mode);

            rect.render_widget(input, chunks[2]);

            let input_mode = app.get_input_mode();

            match input_mode {
                InputMode::Normal =>
                    // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
                    {}

                InputMode::Editing => {
                    // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
                    rect.set_cursor(
                        // Put cursor past the end of the input text
                        chunks[2].x + app.input_string_width() as u16 + 1,
                        // Move one line down, from the border to the input line
                        chunks[2].y + 1,
                    )
                }
            }

            // HACK
            // we really need to use a stateful widget and scroll here, but for the moment,
            // we will just truncate from the front of the messages when there are more
            // of them than can fit on screen (this will work weird with wrap btw)
            let num_ui_lines = (chunks[1].bottom() - chunks[1].top()) as usize;
            if num_ui_lines < app.num_messages() {
                let num_messages_to_drop = app.num_messages() - num_ui_lines;

                app.drain_messages(num_messages_to_drop);
            }

            let message_list = app
                .message_list()
                .iter()
                .cloned()
                .map(|s| {
                    let content = vec![Spans::from(Span::raw(s))];
                    ListItem::new(content)
                })
                .collect::<Vec<_>>();

            let message_list = List::new(message_list)
                .block(Block::default().borders(Borders::ALL).title("Messages"));

            let menu = menu_titles
                .iter()
                .map(|t| {
                    let (first, rest) = t.split_at(1);
                    Spans::from(vec![
                        Span::styled(
                            first,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                })
                .collect();

            let tabs = Tabs::new(menu)
                .select(app.get_active_menu_item().into())
                .block(Block::default().title("Menu").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(Span::raw("|"));

            match app.get_active_menu_item() {
                MenuItem::Home => {
                    rect.render_widget(message_list, chunks[1]);
                }
                MenuItem::Logs => {
                    let tui_w: TuiLoggerWidget = TuiLoggerWidget::default()
                        .block(
                            Block::default()
                                .title("Independent Tui Logger View")
                                .border_style(Style::default().fg(Color::White).bg(Color::Black))
                                .borders(Borders::ALL),
                        )
                        .style(Style::default().fg(Color::White).bg(Color::Black));
                    rect.render_widget(tui_w, chunks[1]);
                }
            }

            rect.render_widget(tabs, chunks[0]);

            rect.render_widget(Paragraph::new("THIS IS CHUNK 3"), chunks[3]);
        })
        .unwrap();
}

fn get_input_box(input_string: String, input_mode: InputMode) -> Paragraph<'static> {
    Paragraph::new(input_string)
        .style(match input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).title("Input"))
}

fn get_input_mode_message(input_mode: InputMode) -> (Vec<Span<'static>>, Style) {
    match input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to start editing."),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to record the message"),
            ],
            Style::default(),
        ),
    }
}

fn build_chunks(size: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(3),
                Constraint::Length(5),
            ]
            .as_ref(),
        )
        .split(size)
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum MenuItem {
    Home,
    Logs,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::Logs => 1,
        }
    }
}

#[tokio::main]
async fn start_peer(
    app: App,
    client: Client,
    server: Server,
    tracker_ip_address: String,
    tracker_port: u16,
    max_incoming_connections: u64,
    mut ui_rx: UiClientRxChannel,
) {
    trace!("Starting peer...");
    tokio::spawn(async move {
        server
            .start(
                &tracker_ip_address,
                &tracker_port.to_string(),
                max_incoming_connections,
            )
            .await
            .unwrap_or_else(|e| {
                debug!("Server shut down: {}", e);
            });
    });

    let moved_client = client.clone();
    tokio::spawn(async move {
        loop {
            if let Some(message_from_ui) = ui_rx.recv().await {
                trace!("Received {:?} from Ui", message_from_ui);
                match message_from_ui {
                    UiClientMessage::Say(message) => {
                        moved_client.say(message).await;
                    },
					UiClientMessage::Whisper(to, message) => {
						moved_client.whisper(to, message).await;
					},
                }
            }
        }
    });

    let x = tokio::spawn(async move {
        loop {
            trace!("Waiting for next message from client");
            let from_server_message = client.next_message().await;

            match from_server_message {
                ClientServerMessage::Say((from, msg)) => {
                    let formatted_say = format!(
                        "[{}] {}: {}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        msg
                    );
                    app.add_message(&formatted_say);
                },
				ClientServerMessage::Whisper((from, to, msg)) => {
					let formatted_say = format!(
						"[{}] {} whispered to {}: {}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
						to,
                        msg
					);
                    app.add_message(&formatted_say);
				},
            }
        }
    });

    x.await.unwrap();
}

#[derive(Debug, Clone)]
pub struct App {
    shared: Arc<Shared>,
}

impl App {
    pub fn new() -> Self {
        let shared = Shared::new();
        Self {
            shared: Arc::new(shared),
        }
    }

    pub(crate) fn get_input_mode(&self) -> InputMode {
        self.shared.state.lock().unwrap().input_mode
    }

    pub fn input_string_push(&self, c: char) {
        self.shared.state.lock().unwrap().input_string.push(c);
    }

    pub fn input_string_pop(&self) -> Option<char> {
        self.shared.state.lock().unwrap().input_string.pop()
    }

    pub fn input_string_drain(&self) -> String {
        self.shared
            .state
            .lock()
            .unwrap()
            .input_string
            .drain(..)
            .collect::<String>()
    }

    /// Add a message that will be displayed in the main ui frame.
    pub fn add_message(&self, message: &str) {
        self.shared
            .state
            .lock()
            .unwrap()
            .messages
            .push_back(message.to_string())
    }

    /// Get the number of messages we have in our buffer
    pub fn num_messages(&self) -> usize {
        self.shared.state.lock().unwrap().messages.len()
    }

    /// Switch between input modes (edit and regular)
    pub(crate) fn set_input_mode(&self, input_mode: InputMode) {
        self.shared.state.lock().unwrap().input_mode = input_mode;
    }

    pub(crate) fn get_active_menu_item(&self) -> MenuItem {
        self.shared.state.lock().unwrap().active_menu_item
    }

    pub(crate) fn set_active_menu_item(&self, menu_item: MenuItem) {
        self.shared.state.lock().unwrap().active_menu_item = menu_item
    }

    pub fn drain_messages(&self, n: usize) {
        drop(self.shared.state.lock().unwrap().messages.drain(..n))
    }

    pub fn input_string_width(&self) -> usize {
        self.shared.state.lock().unwrap().input_string.width()
    }

    pub fn clone_input_string(&self) -> String {
        self.shared.state.lock().unwrap().input_string.clone()
    }

    pub fn is_shutdown(&self) -> bool {
        self.shared.state.lock().unwrap().shut_down
    }

    pub fn shutdown(&self) {
        self.shared.state.lock().unwrap().shut_down = true;
    }

    pub fn message_list(&self) -> Vec<String> {
        let messages = &self.shared.state.lock().unwrap().messages;

        messages
            .iter()
            .enumerate()
            .map(|(i, v)| format!("{}: {}", i, v))
            .collect()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
}

impl Shared {
    pub fn new() -> Self {
        let state = State::new();

        Self {
            state: Mutex::new(state),
        }
    }
}

#[derive(Debug)]
struct State {
    messages: VecDeque<String>,
    input_mode: InputMode,
    active_menu_item: MenuItem,
    input_string: String,
    shut_down: bool,
}

impl State {
    fn new() -> Self {
        State {
            messages: VecDeque::new(),
            input_mode: InputMode::Normal,
            active_menu_item: MenuItem::Home,
            input_string: String::new(),
            shut_down: false,
        }
    }
}
