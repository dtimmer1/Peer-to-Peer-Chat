use crate::{parse::Parse, peer_map::PeerMap, Bing2BingError, Bing2BingFrame};

use bytes::Bytes;

/// `Deliver` data [Bing2BingFrame::Bulk] to a specific destination (peer).
/// # Points available
/// The current implementation of [`Deliver::apply()`] forwards to all connected peers.
/// To receive additional points, you should make an attempt to determine the shortest path
/// and forward _only_ to the next hop in that path.
#[derive(Debug, Clone)]
pub struct Deliver {
    pub(crate) source: String,
    pub(crate) sequence_number: u64,
    destination: String,
    data: Bytes,
}

impl Deliver {
    pub(crate) fn parse_frames(parse: &mut Parse) -> Result<Self, Bing2BingError> {
        let source = parse.next_string()?;

        let sequence_number = parse.next_number()?;

        let destination = parse.next_text()?;

        let data = parse.next_bytes()?;

        parse.finish()?;

        Ok(Self {
            source,
            sequence_number,
            destination,
            data,
        })
    }

    /// Forward the data on to the next peer in the path.
    /// # Points available
    /// Current implementation just broadcasts the command out to all connected peers.
    pub(crate) async fn apply(&self, peer_map: &PeerMap) -> Result<(), Bing2BingError> {
        let frame = self.clone().into_frame();

        peer_map.broadcast(self.source.clone(), frame);

        Ok(())
    }

    /// Turns this `Deliver` into a [Bing2BingFrame].
    pub fn into_frame(self) -> Bing2BingFrame {
        // note that using the vec! macro like this is more
        // performant than creating a new vector and then
        // pushing into it according to clippy:
        // https://rust-lang.github.io/rust-clippy/master/index.html#vec_init_then_push
        let cmd = vec![
            Bing2BingFrame::Text("deliver".to_string()),
            Bing2BingFrame::Text(self.source),
            Bing2BingFrame::Number(self.sequence_number),
            Bing2BingFrame::Text(self.destination),
            Bing2BingFrame::Bulk(self.data.to_vec()),
        ];

        // cmd.push(Bing2BingFrame::Text("deliver".to_string()));
        // cmd.push(Bing2BingFrame::Text(self.source));
        // cmd.push(Bing2BingFrame::Number(self.sequence_number));
        // cmd.push(Bing2BingFrame::Text(self.destination));
        // cmd.push(Bing2BingFrame::Bulk(self.data.to_vec()));

        Bing2BingFrame::Array(cmd)
    }
}
