use structopt::StructOpt;

use std::net::Ipv4Addr;

mod simple_tui;

mod fancy_tui;

#[derive(Debug, StructOpt, Clone)]
pub struct Cli {
    /// What name shoudl this server have?
    #[structopt(long = "name", short = "-N")]
    name: String,
    /// server ip address (0.0.0.0 should be any ip the server can listen on)
    #[structopt(long = "host", short = "-S")]
    ip_address: Ipv4Addr,

    /// server port address
    #[structopt(short, long)]
    port: u16,

    /// tracker ip address
    #[structopt(long = "tracker-host", short = "-T")]
    tracker_ip_address: Ipv4Addr,

    /// tracker port
    #[structopt(short, long)]
    tracker_port: u16,

    /// maximum number of incomming connections that will be advertised when Announcing to the network.
    #[structopt(default_value = "2")]
    max_connections: u64,

    /// Use simple ui mode? (/say and /quit are the only things that work)
    #[structopt(short, long)]
    simple: bool,
}

#[tokio::main]
pub async fn main() -> Result<(), libb2b::Bing2BingError> {
    // simple_ui::do_it().await

    let args = Cli::from_args();

    if args.simple {
        simple_tui::start(args).await
    } else {
        fancy_tui::start(args).await
    }
}

#[derive(Debug)]
pub enum UiClientMessage {
    Say(String),
	Whisper(String, String),
}
