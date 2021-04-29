use crate::{parse::Parse, peer_map::PeerMap, Bing2BingError, Bing2BingFrame};

use bytes::Bytes;

/// The `Broadcast` command delivers data (a [Bing2BingFrame::Bulk]) to all connected peers.
#[derive(Debug, Clone)]
pub struct Broadcast {
    pub(crate) source: String,
    pub(crate) sequence_number: u64,
    data: Bytes,
}

impl Broadcast {
    pub(crate) fn parse_frames(parse: &mut Parse) -> Result<Self, Bing2BingError> {
        let source = parse.next_string()?;

        let sequence_number = parse.next_number()?;

        let data = parse.next_bytes()?;
        parse.finish()?;

        Ok(Self {
            source,
            sequence_number,
            data,
        })
    }

    /// Forwards this command out to all connected peers.
    pub(crate) async fn apply(&self, peer_map: &PeerMap) -> Result<(), Bing2BingError> {
        let frame = self.clone().into_frame();
        peer_map.broadcast(self.source.clone(), frame);

        Ok(())
    }

    /// Turns this `Broadcast` into a [Bing2BingFrame].
    pub fn into_frame(self) -> Bing2BingFrame {
        // note that using the vec! macro like this is more
        // performant than creating a new vector and then
        // pushing into it according to clippy:
        // https://rust-lang.github.io/rust-clippy/master/index.html#vec_init_then_push
        let cmd = vec![
            Bing2BingFrame::Text("broadcast".to_string()),
            Bing2BingFrame::Text(self.source),
            Bing2BingFrame::Number(self.sequence_number),
            Bing2BingFrame::Bulk(self.data.to_vec()),
        ];

        // cmd.push(Bing2BingFrame::Text("broadcast".to_string()));
        // cmd.push(Bing2BingFrame::Text(self.source));
        // cmd.push(Bing2BingFrame::Number(self.sequence_number));
        // cmd.push(Bing2BingFrame::Bulk(self.data.to_vec()));

        Bing2BingFrame::Array(cmd)
    }
}
