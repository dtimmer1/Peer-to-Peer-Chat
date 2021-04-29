use crate::{parse::Parse, peer_map::PeerMap, Bing2BingError, Bing2BingFrame};

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
}

impl Whisper {
    pub fn new(source: String, sequence_number: u64, destination: &str, message: &str) -> Self {
        let destination = destination.to_string();
        let message = message.to_string();

        Self {
            source,
            sequence_number,
            destination,
            message,
        }
    }

    pub(crate) fn parse_frames(parse: &mut Parse) -> Result<Self, Bing2BingError> {
        let source = parse.next_string()?;

        let sequence_number = parse.next_number()?;
        let destination = parse.next_text()?;

        let message = parse.next_text()?;

        parse.finish()?;

        Ok(Self::new(source, sequence_number, &destination, &message))
    }

    /// Currently just broadcasts the message back out to everyone else
    /// This will (eventually) mean that the whisper will arrive at its
    /// destination.
    #[instrument(level = "trace")]
    #[instrument]
    pub(crate) async fn apply(&self, peer_map: &PeerMap) -> Result<(), Bing2BingError> {
        trace!("Applying Whisper command: {:?}", self);

        let frame = self.clone().into_frame();

        peer_map.broadcast(self.source.clone(), frame);

        Ok(())
    }

    pub fn into_frame(self) -> Bing2BingFrame {
        let cmd = vec![
            Bing2BingFrame::Text("whisper".to_string()),
            Bing2BingFrame::Text(self.source),
            Bing2BingFrame::Number(self.sequence_number),
            Bing2BingFrame::Text(self.message),
        ];

        Bing2BingFrame::Array(cmd)
    }
}
