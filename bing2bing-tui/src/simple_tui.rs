use tracing::{debug, instrument, trace};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

use tokio::sync::mpsc;

use libb2b::Server;

use libb2b::ClientServerMessage;

use libb2b::Client;

use tokio::io::AsyncWriteExt;

use chrono::Local;

use crate::Cli;

type UiClientTxChannel = mpsc::UnboundedSender<UiClientMessage>;
type UiClientRxChannel = mpsc::UnboundedReceiver<UiClientMessage>;

pub async fn start(args: Cli) -> Result<(), libb2b::Bing2BingError> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::FULL)
        .init();

    println!("Starting simple ui with args: {:?}", args);

    let ip_address = args.ip_address.to_string().clone();
    let port = args.port;

    let tracker_ip_address = args.tracker_ip_address.to_string().clone();
    let tracker_port = args.tracker_port;

    let my_name = args.name;

    let max_connections = args.max_connections;

    // *POINTS AVAILABLE*
    // I think this stuff can be refactored to be nicer
    let (ui_client_tx, ui_client_rx) = mpsc::unbounded_channel();

    let (client, server) = libb2b::init(&my_name, &ip_address, port).await;

    let network_client = client.clone();
    std::thread::spawn(move || {
        start_peer(
            my_name,
            network_client,
            server,
            tracker_ip_address,
            tracker_port,
            max_connections,
            ui_client_rx,
        )
    });

    start_ui(App {}, ui_client_tx.clone()).await;

    Ok(())
}

#[derive(Debug, Clone)]
pub enum UiClientMessage {
    Say(String),
	Whisper(String, String),
}

#[tokio::main]
#[instrument(level = "trace")]
async fn start_peer(
    my_name: String,
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

            let mut stdout = tokio::io::stdout();

            match from_server_message {
                ClientServerMessage::Say((from, msg)) => {
                    let formatted_say = format!(
                        "[{}] {}: {}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
                        msg
                    );
                    stdout.write_all(formatted_say.as_bytes()).await.unwrap();
                    stdout.flush().await.unwrap();
                },
				ClientServerMessage::Whisper((from, to, msg)) => {
					let formatted_say = format!(
						"[{}] {} whispered to {}: {}\n",
                        Local::now().format("%Y-%m-%d %H:%M:%S"),
                        from,
						to,
                        msg
					);
                    stdout.write_all(formatted_say.as_bytes()).await.unwrap();
                    stdout.flush().await.unwrap();
				},
            }
        }
    });

    x.await.unwrap();
}

#[instrument(level = "trace")]
async fn start_ui(app: App, client_tx: UiClientTxChannel) {
    let ui_input = tokio::spawn(async move {
        let stdin = std::io::stdin();

        let stdin = stdin.lock();

        for line in std::io::BufRead::lines(stdin) {
            let line = line.unwrap();
            trace!("ui had {:?} entered by user!", line);

            if line == *"/quit" {
                break;
            }

            if line.starts_with("/say ") {
                trace!("input line started with /say !");
                let line = line.clone().strip_prefix("/say ").unwrap().to_string();

                let client_tx = client_tx.clone();

                let msg = UiClientMessage::Say(line);
	
                trace!("Ui thread sending {:?} over client channle", msg);
                client_tx.send(msg).unwrap();
            }
			if line.starts_with("/whisper ") {
				trace!("input line started with /whisper !");
				let string = line.clone().strip_prefix("/whisper ").unwrap().to_string();
				let splitter: Vec<&str> = string.splitn(2, ' ').collect();
				let client_tx = client_tx.clone();
				if splitter.len() == 2{
					let msg = UiClientMessage::Whisper(splitter[0].to_string(), splitter[1].to_string());
					client_tx.send(msg).unwrap();
				}
			}
        }
    });

    ui_input.await.unwrap();
}

#[derive(Debug)]
struct App {}
