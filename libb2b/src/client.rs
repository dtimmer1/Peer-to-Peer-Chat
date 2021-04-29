use std::sync::Arc;

use tracing::{instrument, trace};

use crate::ClientServerMessage;
use crate::{ClientRxChannel, ServerTxChannel};

/// A `Client` is the way that a user (i.e., a user of our crate) interacts with a [Server](crate::Server),
/// and thus the rest of the network..
#[derive(Debug, Clone)]
pub struct Client {
    shared: Arc<Shared>,
}

impl Client {
    #[instrument(level = "trace")]
    pub fn new(name: String, server_tx: ServerTxChannel, rx: ClientRxChannel) -> Client {
        Self {
            shared: Arc::new(Shared::new(name, server_tx, rx)),
        }
    }

    /// A method for use by users of `Client` to say a message that will be
    /// broadcast through the network.
    #[instrument(level = "trace")]
    pub async fn say(&self, message: String) {
        let message = ClientServerMessage::Say((self.shared.name.clone(), message));

        // pass the message on to the server
        self.shared.server_tx.send(message).await.unwrap();
    }

    /// Get the next message that came from the server.
    /// I.e., an already processed message that the user of
    /// the client might be interested in looking at.
    /// E.g., a message that came from another user that should be displayed
    /// in a UI.
    #[instrument(level = "trace")]
    pub async fn next_message(&self) -> ClientServerMessage {
        loop {
            if let Ok(msg) = self.shared.rx.recv().await {
                trace!("Received a ClientServerMessage: {:?}", msg);
                return msg;
            }
        }
    }
}

#[derive(Debug)]
struct Shared {
    name: String,
    server_tx: ServerTxChannel,
    rx: ClientRxChannel,
}

impl Shared {
    #[instrument(level = "trace")]
    pub fn new(name: String, server_tx: ServerTxChannel, rx: ClientRxChannel) -> Self {
        Self {
            name,
            server_tx,
            rx,
        }
    }
}
