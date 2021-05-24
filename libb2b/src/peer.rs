use crate::PeerRxChannel;
use crate::{Bing2BingError, Connection, PeerControlMessage};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::cmp::Ordering;
use tokio::net::TcpStream;

pub(crate) struct Peer {
    info: PeerInfo,
    rx: PeerRxChannel,
}

impl Peer {
    pub(crate) fn new(name: String, ip_address: String, port: String, rx: PeerRxChannel) -> Self {
        let addr: SocketAddr = format!("{}:{}", ip_address, port).parse().unwrap();
        Peer {
            info: PeerInfo { name, addr },
            rx,
        }
    }

    pub(crate) async fn start(&mut self) -> Result<(), Bing2BingError> {
        let tcp_stream = TcpStream::connect(self.info.addr).await.unwrap();

        let mut connection = Connection::new(tcp_stream).await;

        loop {
            // let x = self.rx.recv().await;
            tokio::select! {
                Some(control_message) = self.rx.recv() => {
                    // we received something, send it across the network.
                    match control_message {
                        PeerControlMessage::Frame(frame) => {
                            connection.write_frame(frame).await?;
                        },
                        PeerControlMessage::ShutDown => {
                            break;
                        }
                    }

                },
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub(crate) struct PeerInfo {
    name: String,
    addr: SocketAddr,
}

/// POINTS AVAILABLE FOR CLEANING THIS UP (renaming/refactoring as needed?)
/// This is a very poorly named structure that wraps the bits of data
/// that come in over an [Announce](crate::cmd::Announce).
#[derive(Debug, Clone)]
pub struct PeerData {
    city: String,
    lat: f64,
    lng: f64,
    peers: Vec<(String, u32)>,
}

impl PeerData {
    pub fn new(city: &str, lat: f64, lng: f64, peers: Vec<(String, u32)>) -> Self {
        Self {
            city: city.to_string(),
            lat,
            lng,
            peers,
        }
    }

	pub fn get_peers(&self) -> &Vec<(String, u32)> {
		&(self.peers)
	}
}
