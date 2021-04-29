use std::time::Duration;

use crate::{util::TtlMap, Bing2BingError, Bing2BingFrame, Parse};

mod ping;

pub use ping::Ping;

mod say;
pub use say::Say;

mod register;
pub use register::Register;

mod announce;
pub use announce::Announce;

mod whisper;
pub use whisper::Whisper;

mod broadcast;
pub use broadcast::Broadcast;

mod deliver;
pub use deliver::Deliver;

mod extension;
pub use extension::Extension;

#[derive(Debug)]
pub enum Bing2BingCommand {
    Broadcast(Broadcast),
    Ping(Ping),
    Register(Register),
    Say(Say),
    Deliver(Deliver),
    Announce(Announce),
    Whisper(Whisper),
    Extension(Extension),
    Unknown,
}

impl Bing2BingCommand {
    pub(crate) fn from_frame(frame: Bing2BingFrame) -> Result<Bing2BingCommand, Bing2BingError> {
        let mut parse = Parse::new(frame)?;

        let command_name = parse.next_string()?.to_lowercase();

        let command = match &command_name[..] {
            "broadcast" => Bing2BingCommand::Broadcast(Broadcast::parse_frames(&mut parse)?),
            "ping" => Bing2BingCommand::Ping(Ping::parse_frames(&mut parse)?),
            "register" => Bing2BingCommand::Register(Register::parse_frames(&mut parse)?),
            "say" => Bing2BingCommand::Say(Say::parse_frames(&mut parse)?),
            "deliver" => Bing2BingCommand::Deliver(Deliver::parse_frames(&mut parse)?),
            "announce" => {
                // POINTS AVAILABLE
                // I made the announce command work differently from all the other commands
                // on purpose. Theoretically, I think that because all commands have
                // a source and a sequence number that we could just parse out for every
                // command instead of doing it in each of their parse_frames implementation
                let source = parse.next_string()?;
                // debug!("getting Announce sequence_number");
                let sequence_number = parse.next_number()?;
                // debug!("parsing rest of frame");
                Bing2BingCommand::Announce(Announce::parse_frames(
                    source,
                    sequence_number,
                    &mut parse,
                )?)
            }
            "whisper" => Bing2BingCommand::Whisper(Whisper::parse_frames(&mut parse)?),
            "extension" => Bing2BingCommand::Extension(Extension::parse_frames(&mut parse)?),
            _ => return Ok(Bing2BingCommand::Unknown),
        };

        parse.finish()?;

        Ok(command)
    }

    /// POINTS AVAILABLE
    /// There is a way to refactor things such that we could do a call like
    /// `Bing2BingCommand::into_frame(cmd)` instead of having to call
    /// `cmd.into_frame()` directly. This would give us some benefits with
    /// respect to ergonomics (we wouldn't have to have the underlying cmd struct)
    /// fully typed.
    pub fn into_frame(cmd: Bing2BingCommand) -> Bing2BingFrame {
        match cmd {
            Bing2BingCommand::Ping(ping) => ping.into_frame(),
            _ => todo!(),
        }
    }

    /// Checks to make sure that this `Bing2BingCommand` hasn't already been processed.
    /// This helps us ensure that we don't start an infinite loop.
    pub(crate) fn check_duplicate(&self, processed_commands: &TtlMap<bool>) -> bool {
        let (source, sequence_number) = match self {
            Bing2BingCommand::Announce(announce) => (&announce.source, announce.sequence_number),
            Bing2BingCommand::Broadcast(broadcast) => {
                (&broadcast.source, broadcast.sequence_number)
            }
            Bing2BingCommand::Deliver(deliver) => (&deliver.source, deliver.sequence_number),
            Bing2BingCommand::Register(register) => (&register.peer_name, register.sequence_number),
            Bing2BingCommand::Unknown => return false,
            Bing2BingCommand::Ping(ping) => (&ping.source, ping.sequence_number),
            Bing2BingCommand::Say(say) => (&say.source, say.sequence_number),
            Bing2BingCommand::Whisper(whisper) => (&whisper.source, whisper.sequence_number),
            Bing2BingCommand::Extension(extension) => {
                (&extension.source, extension.sequence_number)
            }
        };

        processed_commands
            .get(&format!("{}-{}", source, sequence_number))
            .is_some()
    }

    pub(crate) fn set_processed(&self, processed_commands: &TtlMap<bool>) {
        let (source, sequence_number) = match self {
            Bing2BingCommand::Announce(announce) => (&announce.source, announce.sequence_number),
            Bing2BingCommand::Broadcast(broadcast) => {
                (&broadcast.source, broadcast.sequence_number)
            }
            Bing2BingCommand::Deliver(deliver) => (&deliver.source, deliver.sequence_number),
            Bing2BingCommand::Register(register) => (&register.peer_name, register.sequence_number),
            Bing2BingCommand::Unknown => return,
            Bing2BingCommand::Ping(ping) => (&ping.source, ping.sequence_number),
            Bing2BingCommand::Say(say) => (&say.source, say.sequence_number),
            Bing2BingCommand::Whisper(whisper) => (&whisper.source, whisper.sequence_number),
            Bing2BingCommand::Extension(extension) => {
                (&extension.source, extension.sequence_number)
            }
        };

        processed_commands.set(
            format!("{}-{}", source, sequence_number),
            true,
            Some(Duration::from_secs(30)),
        );
    }
}
