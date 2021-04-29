use tokio::sync::mpsc;

use std::net::SocketAddr;
use tokio::net::TcpStream;

use tokio::net::TcpListener;

use std::time::Duration;

use crate::{
    cmd::{Announce, Say},
    peer::PeerData,
    util::{ConnectionCounter, SequenceNumberGenerator},
    ClientServerMessage, ClientTxChannel, Peer, ServerRxChannel,
};

use tracing::{debug, instrument, trace};

use crate::cmd::Register;
use crate::Bing2BingError;
use crate::{
    parse::{Parse, ParseError},
    peer_map::PeerMap,
    Bing2BingFrame, Connection,
};
use crate::{util::TtlMap, Bing2BingCommand};

/// The "server" side of the P2P chat application.
/// A server is primarily focused around network related activity and manages most everything related to the protocol itself.
/// This includes receiving commands over the network, processing them, and sending commands out to the network.
/// The server also receives messages from its corresponding [Client](crate::Client) which is what the end user will be interacting with.
#[derive(Debug)]
pub struct Server {
    listener: TcpListener,
    sequence_numbers: SequenceNumberGenerator,
    name: String,
    ip_address: String,
    port: u64,
    num_incoming_conns: ConnectionCounter,
    client_tx: ClientTxChannel,
    rx: ServerRxChannel,
}

impl Server {
    pub async fn new(
        name: &str,
        bind_address: &str,
        port: &str,
        client_tx: ClientTxChannel,
        rx: ServerRxChannel,
    ) -> Result<Self, Bing2BingError> {
        Ok(Server {
            listener: TcpListener::bind(format!("{}:{}", bind_address, port)).await?,
            sequence_numbers: SequenceNumberGenerator::new(0),
            name: name.to_string(),
            ip_address: bind_address.to_string(),
            port: port.to_string().parse().unwrap(),
            num_incoming_conns: ConnectionCounter::new(0),
            client_tx,
            rx,
        })
    }

    /// Begin listening for inbound connections.
    #[instrument(level = "trace")]
    pub async fn listen(
        &self,
        peer_map: &PeerMap,
        client_tx: ClientTxChannel,
    ) -> Result<(), Bing2BingError> {
        let peers = peer_map;
        let adjacency_list: TtlMap<PeerData> = TtlMap::new();
        let processed_commands: TtlMap<bool> = TtlMap::new();

        loop {
            let (stream, addr) = self.listener.accept().await?;

            let peers = peers.clone(); //Arc::clone(&peers);
            let adjacency_list = adjacency_list.clone();

            let processed_commands = processed_commands.clone();
            let connection_counter = self.num_incoming_conns.clone();

            let client_tx = client_tx.clone();

            tokio::spawn(async move {
                trace!("Accepted connection from {:?}", addr);

                connection_counter.inc();

                let connection_handler = Server::handle_connection(
                    &peers,
                    adjacency_list,
                    processed_commands,
                    stream,
                    addr,
                    client_tx,
                );

                connection_handler.await.unwrap_or_else(|err| {
                    trace!(
                        "An error occurred: {:?}; incoming connection disconnected?",
                        err
                    );
                });

                connection_counter.dec();
            });
        }
    }

    /// Handles an incomming connection. I.e., another peer that has initiated a connection with us.
    /// In particular, this method reads command frames from a [Connection], checks to make sure that the
    /// command hasn't already been processed, and if it hasn't, processes the command.
    /// This method will also pass relevant [ClientServerMessage]s up to a
    /// [Client](crate::Client) for further use.
    #[instrument(level = "trace")]
    pub async fn handle_connection(
        peers: &PeerMap,
        adjacency_list: TtlMap<PeerData>,
        processed_commands: TtlMap<bool>,
        stream: TcpStream,
        addr: SocketAddr,
        client_tx: ClientTxChannel,
    ) -> Result<(), Bing2BingError> {
        let mut connection = Connection::new(stream).await;

        loop {
            let frame = connection.read_frame().await?;
            trace!("Received {:?} from {}", frame, addr);

            let frame = match frame {
                Some(frame) => frame,
                None => {
                    trace!("Connection ended?");
                    break;
                }
            };

            // we expect to only see Command frames at this point.
            let command = Bing2BingCommand::from_frame(frame)?;

            trace!(?command);

            // let's see if we've already processed this commmand.
            if command.check_duplicate(&processed_commands) {
                continue;
            }

            command.set_processed(&processed_commands);

            // now see which command it was and apply it.
            // this could be refactored to another function to make life easier
            // perhaps?
            match command {
                Bing2BingCommand::Ping(cmd) => cmd.apply(&mut connection).await?,
                Bing2BingCommand::Say(cmd) => {
                    trace!("Received a Say command on an incoming connection");
                    trace!("Sending to client");
                    client_tx
                        .send(ClientServerMessage::Say((
                            cmd.source.clone(),
                            cmd.message.clone(),
                        )))
                        .await?;
                    cmd.apply(&peers).await?;
                }
                Bing2BingCommand::Announce(cmd) => cmd.apply(&peers, &adjacency_list).await?,
                Bing2BingCommand::Broadcast(cmd) => cmd.apply(&peers).await?,
                Bing2BingCommand::Deliver(cmd) => cmd.apply(&peers).await?,
                Bing2BingCommand::Whisper(cmd) => cmd.apply(&peers).await?,
                Bing2BingCommand::Extension(cmd) => cmd.apply(&peers).await?,
                Bing2BingCommand::Register(cmd) => {
                    tracing::error!(
                        "REGISTER COMMAND NOT IMPLEMENTED BY DEFAULT ON SERVERS (peers) {:?}",
                        cmd
                    )
                }
                Bing2BingCommand::Unknown => {
                    tracing::trace!("Received unimplemented command! {:?}", command)
                }
            }
        }

        Ok(())
    }

    /// Convienence function that broadcasts a say message.
    /// This is useful for handling messages that are coming in from the associated [Client](crate::Client).
    /// I.e., our user wants to say something.
    pub async fn say(peer_map: &PeerMap, from: String, message: String, sequence_number: u64) {
        let frame = Say::new(from.to_string(), sequence_number, &message).into_frame();

        peer_map.broadcast(from, frame);
    }

    /// Convienence function that gets the next sequence number for a message originating from this peer.
    fn next_sequence_number(&self) -> u64 {
        self.sequence_numbers.next()
    }

    /// Starts the server.
    /// This is primarily three steps:
    ///
    /// 1. We want to register with the tracker.
    /// 2. We want to connect to peers that we get back from the tracker.
    /// 3. We want to start listening for incoming connections from other peers.
    /// 4. We want to start announcing our neighborhood to others.
    #[instrument(level = "trace")]
    pub async fn start(
        &self,
        tracker_ip: &str,
        tracker_port: &str,
        max_incoming_connections: u64,
        // next_sequence_number: Arc<Mutex<u64>>,
    ) -> Result<(), Bing2BingError> {
        // 1) we want to connect to tracker.
        // 2) we want to connect to peers
        // 3) we want to start listening for incoming connections.

        // Connect to tracker
        let tracker_addr = format!("{}:{}", tracker_ip, tracker_port);
        let tcp_stream = TcpStream::connect(tracker_addr).await?;
        let mut connection = Connection::new(tcp_stream).await;

        let sequence_number = self.next_sequence_number();

        let frame = Register::new(
            &self.name,
            sequence_number,
            &self.ip_address,
            &self.port.to_string(),
        )
        .into_frame();

        // lock is released; we have a "guaranteed unique" sequence number
        connection.write_frame(frame).await.unwrap();

        let response_frame = connection.read_frame().await.unwrap().unwrap();
        let received_peers = self.parse_register_response(response_frame)?;
        trace!("received peers from announce: {:?}", received_peers);
        let peer_map = PeerMap::default();

        // we need to add each of these to the peer map.
        for (peer_name, ip_address, port) in received_peers {
            trace!("Adding peer {} from Register list", peer_name);
            if peer_name != self.name {
                Server::connect_to_peer(&peer_map, peer_name, ip_address, port);
            }
        }

        // let next_sequence_number = self.sequence_numbers.clone();
        let peer_map_move = peer_map.clone();

        self.client_message_handler(&peer_map_move, self.rx.clone());

        // start up an announce task
        let next_sequence_number = self.sequence_numbers.clone();
        let peer_map_move = peer_map.clone();

        let name = self.name.clone();
        // let port = self.port;
        let ip_address = self.ip_address.clone();

        let port = self.port;

        let num_incoming_conns = self.num_incoming_conns.clone();

        // POINTS AVAILABLE
        // this might be fine just doing a tokio spawn instead of a thread.
        std::thread::spawn(move || {
            start_announce(
                name,
                ip_address,
                port,
                &peer_map_move,
                next_sequence_number,
                num_incoming_conns,
                max_incoming_connections,
            )
        });

        self.listen(&peer_map, self.client_tx.clone()).await
    }

    /// This method handles messages that come in from the associated [Client](crate::Client)
    #[instrument(level = "trace")]
    fn client_message_handler(&self, peer_map: &PeerMap, rx: ServerRxChannel) {
        let peer_map = peer_map.clone();
        let next_sequence_number = self.sequence_numbers.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(msg) = rx.recv().await {
                    trace!("Server received {:?} from client", msg);
                    match msg {
                        ClientServerMessage::Say((from, message)) => {
                            trace!("matched a  ClientServerMessage::Say message");
                            // we should do a say.
                            let sequence_number = next_sequence_number.next();

                            trace!("exceutiong Server::say");

                            Server::say(&peer_map, from, message, sequence_number).await;
                        }
                    }
                }
            }
        });
    }

    fn parse_register_response(
        &self,
        response: Bing2BingFrame,
    ) -> Result<Vec<(String, String, String)>, Bing2BingError> {
        let mut parse = Parse::new(response)?;

        let mut ret: Vec<(String, String, String)> = Vec::new();

        loop {
            match parse.next() {
                Ok(Bing2BingFrame::Array(array)) => {
                    // POINTS AVAILABLE
                    // i don't think i should have to deconstruct and then reconstruct
                    // this, although i'm not sure how to deal with it better.
                    let array = Bing2BingFrame::Array(array);

                    let mut peer_info_parse = Parse::new(array)?;

                    let peer_name = peer_info_parse.next_string()?;
                    let ip_address = peer_info_parse.next_string()?;
                    let port = peer_info_parse.next_string()?;

                    peer_info_parse.finish()?;
                    ret.push((peer_name, ip_address, port));
                }
                Err(ParseError::EndOfStream) => break,
                Err(err) => return Err(Box::new(err)),
                _ => {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Found a tracker register response that was not an array!",
                    )))
                }
            }
        }

        Ok(ret)
    }

    #[instrument(level = "trace")]
    pub(crate) fn connect_to_peer(
        peer_map: &PeerMap,
        peer_name: String,
        ip_address: String,
        port: String,
    ) {
        let mut peer_map = peer_map.clone();

        tokio::spawn(async move {
            let (peer_tx, peer_rx) = mpsc::unbounded_channel();

            // POINTS AVAILABLE
            // It is likely possible to remove all these clones with some refactoring, but I got lazy
            let mut peer = Peer::new(peer_name.clone(), ip_address.clone(), port.clone(), peer_rx);

            peer_map.insert(peer_name.clone(), peer_tx);

            peer.start()
                .await
                .unwrap_or_else(|x| debug!("peer.start() errored out {}", x));

            // this is not the greatest way to handle a disconnect coming from a peer
            // but, we could send a [Peer] a [PeerControlMessage::ShutDown] and then it should break from the loop
            peer_map.remove(peer_name.clone());
        });
    }
}

/// *POINTS AVAILABLE*
/// Right now, this method will announce the peer to the rest
/// of the network every 5 seconds.
/// As part of this announcement, the peer will transmit the name of the city
/// it's in, as well as lat and longitude.
/// It would be nice to have this be configurable instead of hard coded
/// as it currently is.
#[tokio::main]
#[instrument(level = "trace")]
async fn start_announce(
    name: String,
    ip_address: String,
    port: u64,
    peer_map: &PeerMap,
    next_sequence_number: SequenceNumberGenerator,
    num_incoming_conns: ConnectionCounter,
    max_incoming_conns: u64,
) {
    loop {
        let sequence_number = next_sequence_number.next();

        let peers = peer_map.peer_names();

        let num_incoming_conns = num_incoming_conns.get();

        let available_incoming = match num_incoming_conns < max_incoming_conns {
            true => max_incoming_conns - num_incoming_conns,
            false => 0,
        };

        // POINTS AVAILABLE
        // Try making a configuration setting that does this with some more
        // configurability
        let announce = Announce::new(
            name.clone(),
            sequence_number,
            ip_address.clone(),
            port,
            available_incoming,
            "New York".to_string(),
            40.6943,
            -73.9249,
            peers,
        );

        let announce_frame = announce.into_frame();
        trace!("Broadcasting announce frame: {:?}", announce_frame);

        peer_map.broadcast(name.clone(), announce_frame);

        trace!("announce sleeping");
        tokio::time::sleep(Duration::from_secs(5)).await;
        trace!("announce woke up!");
    }
}
