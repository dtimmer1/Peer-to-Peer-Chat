use crate::{Bing2BingError, Bing2BingFrame, Connection, Parse};

use tracing::trace;

/// A simple command that let's peers test latency between each other.
#[derive(Debug)]
pub struct Ping {
    pub(crate) source: String,
    pub(crate) sequence_number: u64,
}

impl Ping {
    pub fn new(source: String, sequence_number: u64) -> Self {
        Self {
            source,
            sequence_number,
        }
    }

    /// Returns a parsed Ping command.
    pub(crate) fn parse_frames(parse: &mut Parse) -> Result<Self, Bing2BingError> {
        let source = parse.next_text()?;

        let sequence_number = parse.next_number()?;

        parse.finish()?;

        Ok(Ping::new(source, sequence_number))
    }

    pub(crate) async fn apply(self, dst: &mut Connection) -> Result<(), Bing2BingError> {
        let response = Bing2BingFrame::Number(self.sequence_number);

        trace!(?response);

        dst.write_frame(response).await?;

        Ok(())
    }

    /// Turns this `Ping` into a [Bing2BingFrame].
    pub fn into_frame(self) -> Bing2BingFrame {
        // note that using the vec! macro like this is more
        // performant than creating a new vector and then
        // pushing into it according to clippy:
        // https://rust-lang.github.io/rust-clippy/master/index.html#vec_init_then_push
        let cmd = vec![
            Bing2BingFrame::Text("ping".to_string()),
            Bing2BingFrame::Text(self.source),
            Bing2BingFrame::Number(self.sequence_number),
        ];

        // cmd.push(Bing2BingFrame::Text("ping".to_string()));
        // cmd.push(Bing2BingFrame::Text(self.source));
        // cmd.push(Bing2BingFrame::Number(self.sequence_number));

        Bing2BingFrame::Array(cmd)
    }
}
