use crate::{peer_map::PeerMap, Bing2BingError, Bing2BingFrame, Parse};

use tracing::{instrument, trace};

/// This command allows for propagation of normal chat messages through out the network.
#[derive(Debug, Clone)]
pub struct Say {
    pub(crate) source: String,
    pub(crate) sequence_number: u64,
    pub(crate) message: String,
}

impl Say {
    pub fn new(source: String, sequence_number: u64, message: &str) -> Self {
        let message = message.to_string();

        Self {
            source,
            sequence_number,
            message,
        }
    }

    pub(crate) fn parse_frames(parse: &mut Parse) -> Result<Self, Bing2BingError> {
        let source = parse.next_string()?;

        let sequence_number = parse.next_number()?;

        let message = parse.next_text()?;

        parse.finish()?;

        Ok(Self::new(source, sequence_number, &message))
    }

    #[instrument(level = "trace")]
    pub(crate) async fn apply(&self, peer_map: &PeerMap) -> Result<(), Bing2BingError> {
        trace!("Applying Say command: {:?}", self);
        // let's check to see if we have already covered the sequence number.
        // if we haven't, we will print this message out, if we have, we won't print it

        let frame = self.clone().into_frame();

        peer_map.broadcast(self.source.clone(), frame);

        Ok(())
    }

    /// Turns this `Say` into a [Bing2BingFrame].
    pub fn into_frame(self) -> Bing2BingFrame {
        let cmd = vec![
            Bing2BingFrame::Text("say".to_string()),
            Bing2BingFrame::Text(self.source),
            Bing2BingFrame::Number(self.sequence_number),
            Bing2BingFrame::Text(self.message),
        ];

        // cmd.push(Bing2BingFrame::Text("say".to_string()));
        // cmd.push(Bing2BingFrame::Text(self.source));
        // cmd.push(Bing2BingFrame::Number(self.sequence_number));
        // cmd.push(Bing2BingFrame::Text(self.message));

        Bing2BingFrame::Array(cmd)
    }
}
