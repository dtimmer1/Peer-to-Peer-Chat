use crate::{parse::Parse, peer_map::PeerMap, Bing2BingError, Bing2BingFrame, peer::PeerData, util::TtlMap};

use tracing::{instrument, trace};

/// This command allows for direct messaging between two peers.
/// The idea is that peers should forward this message via the shortest path to the target.
///
/// # Points available.
///
/// Currently, [Whisper::apply()] just treats things as a [Say](crate::cmd::Say), and thus
/// the message is broadcast to all outgoing peers.
/// For extra points, make it only send the data out over the next hop in the shortest path
/// to the destination.
#[derive(Debug, Clone)]
pub struct Whisper {
    pub(crate) source: String,
    pub(crate) sequence_number: u64,
    pub(crate) destination: String,
    pub(crate) message: String,
	pub(crate) path: Vec<String>,
}

impl Whisper {
    pub fn new(source: String, sequence_number: u64, destination: &str, message: &str, path: Vec<String>) -> Self {
        let destination = destination.to_string();
        let message = message.to_string();

        Self {
            source,
            sequence_number,
            destination,
            message,
			path,
        }
    }

    pub(crate) fn parse_frames(parse: &mut Parse) -> Result<Self, Bing2BingError> {
        let source = parse.next_string()?;

        let sequence_number = parse.next_number()?;
        let destination = parse.next_text()?;

        let message = parse.next_text()?;

		let peers = Whisper::parse_path_frames(parse)?;


        parse.finish()?;

        Ok(Self::new(source, sequence_number, &destination, &message, peers))
    }

	fn parse_path_frames(parse: &mut Parse) -> Result<Vec<String>, Bing2BingError> {
        // This should be an array
        let path_frames = parse.next_array()?;

        // We will loop through each element of the array
        // if it is a Text frame, we will assume that is the name of a peer that
        // is in the shortest path.
        let mut ret = vec![];
        for peer_name in path_frames {
            match peer_name {
                Bing2BingFrame::Text(peer_name) => {
                    ret.push(peer_name);
                }
                frame => {
                    return Err(format!(
                    "protocol error; expected text frame when parsing announce peer info, got {:?}",
                    frame
                )
                    .into())
                }
            }
        }

        Ok(ret)
    }


    /// Currently just broadcasts the message back out to everyone else
    /// This will (eventually) mean that the whisper will arrive at its
    /// destination.
    #[instrument(level = "trace")]
//    #[instrument]
    pub(crate) async fn apply(&self, peer_map: &PeerMap) -> Result<(), Bing2BingError> {
        trace!("Applying Whisper command: {:?}", self);

        if(self.path.clone().is_empty()){
			return Ok(());
		}
		
		let next_step = self.path.clone().remove(0);

		let frame = self.clone().into_frame();

        peer_map.send_to_peer(self.source.clone(), next_step, frame);

        Ok(())
    }

    pub fn into_frame(self) -> Bing2BingFrame {
        let mut cmd = vec![
            Bing2BingFrame::Text("whisper".to_string()),
            Bing2BingFrame::Text(self.source),
            Bing2BingFrame::Number(self.sequence_number),
			Bing2BingFrame::Text(self.destination),
            Bing2BingFrame::Text(self.message),
        ];
		let mut peers = vec![];

        for peer in self.path {
            peers.push(Bing2BingFrame::Text(peer));
        }

        cmd.push(Bing2BingFrame::Array(peers));

        Bing2BingFrame::Array(cmd)
    }
}
