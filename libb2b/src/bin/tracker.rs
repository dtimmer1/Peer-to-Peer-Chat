use std::net::Ipv4Addr;
use structopt::StructOpt;

use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

use libb2b::Tracker;

#[derive(Debug, StructOpt, Clone)]
struct Cli {
    /// ip address to bind to (0.0.0.0 should be any ip the tracker can listen on)
    #[structopt(long = "host", short = "-S")]
    ip_address: Ipv4Addr,

    /// The port the tracker should listen on.
    #[structopt(short, long)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), libb2b::Bing2BingError> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::FULL)
        .init();

    let args = Cli::from_args();

    println!("Tracker starting with args: {:?}", args);
    // let's start up a tracker  and listen.

    let ip_address = &args.ip_address.to_string();
    let port = &args.port.to_string();

    let tracker = Tracker::new(&ip_address, &port).await.unwrap();

    tracker.listen().await?;

    Ok(())
}
