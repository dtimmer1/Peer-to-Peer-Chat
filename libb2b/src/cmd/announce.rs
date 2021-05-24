use crate::{
    peer::PeerData, peer_map::PeerMap, util::TtlMap, Bing2BingError, Bing2BingFrame, Parse, Server,
};

use rand::Rng;

use std::time::Duration;

/// The `Announce` command is propagated through the network to provide peers knowledge about the network topography.
/// I.e., this is how peers let each other know who they are connected to.
#[derive(Debug)]
pub struct Announce {
    pub(crate) source: String,
    pub(crate) sequence_number: u64,
    ip_address: String,
    port: u64,
    available_incoming: u64,
    city: String,
    lat: f64,
    lng: f64,
    peers: Vec<(String, u32)>,
}

impl Announce {
    pub fn new(
        source: String,
        sequence_number: u64,
        ip_address: String,
        port: u64,
        available_incoming: u64,
        city: String,
        lat: f64,
        lng: f64,
        peers: Vec<(String, u32)>,
    ) -> Self {
        Self {
            source,
            sequence_number,
            ip_address,
            port,
            available_incoming,
            city,
            lat,
            lng,
            peers,
        }
    }

    pub(crate) fn parse_frames(
        source: String,
        sequence_number: u64,
        parse: &mut Parse,
    ) -> Result<Self, Bing2BingError> {
        let ip_address = parse.next_text()?;
        let port = parse.next_number()?;
        let available_incoming = parse.next_number()?;

        let city = parse.next_text()?;
        let lat = parse.next_float()?;
        let lng = parse.next_float()?;

        let peers = Announce::parse_peer_info_frames(parse)?;

        parse.finish()?;

        Ok(Self {
            source,
            sequence_number,
            ip_address,
            port,
            available_incoming,
            city,
            lat,
            lng,
            peers,
        })
    }

    fn parse_peer_info_frames(parse: &mut Parse) -> Result<Vec<(String, u32)>, Bing2BingError> {
        // This should be an array
        let peer_info_frames = parse.next_array()?;

        // We will loop through each element of the array
        // if it is a Text frame, we will assume that is the name of a peer that
        // the source has an out going connection to.
        let mut ret = vec![];
        for peer in peer_info_frames {
            match peer  {
                Bing2BingFrame::Latency(peer_name, latency) => {
                    ret.push((peer_name, latency));
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

    pub(crate) async fn apply(
        &self,
        peer_map: &PeerMap,
        adjacency_list: &TtlMap<PeerData>,
    ) -> Result<(), Bing2BingError> {
        // let mut peer_map = peer_map.clone();

        let source = self.source.clone();
        let sequence_number = self.sequence_number;
        let ip_address = self.ip_address.clone();
        let port = self.port;
        let available_incoming = self.available_incoming;
        let city = self.city.clone();
        let lat = self.lat;
        let lng = self.lng;
        let peers = self.peers.clone();

        // add the source's neighbors to our local knowledge
        adjacency_list.set(
            self.source.clone(),
            PeerData::new(&city, lat, lng, self.peers.clone()),
            Some(Duration::from_secs(30)),
        );

        // now broadcast the message on to our neigbhors.

        let frame = Self {
            source,
            sequence_number,
            ip_address,
            port,
            available_incoming,
            city,
            lat,
            lng,
            peers,
        }
        .into_frame();

        peer_map.broadcast(self.source.clone(), frame);

		if available_incoming < 1 {
			return Ok(());
		}

        // Now we want to try to perform an opportunistic connection
        // This helps to ensure that as long as we have at least one
        // outgoing connection, that we should eventually get at least
        // one incoming connection too, which ensures that the
        // there will always be a route to every peer in the network

        // we don't want to connect if we already have an outgoing connection to this
        // peer tho!
        if peer_map.contains_peer(self.source.clone()) {
            return Ok(());
        }

        // we should also flip an `available_incoming` sided coin and
        // connect to this Peer if we get a hit.
        let mut rng = rand::thread_rng();
		let roll = rng.gen_range(0..available_incoming);
		if roll == 0 {
			// EXTRA CREDIT
			// this should be refactored into somewhere else
			// not sure where :3

			// we should go ahead and make an outgoing connection to this peer.
			// let mut peer_map = peer_map;

			// EXTRA CREDIT
			// all these clones are perhaps expensive.
			// there is a better way to deal many of them I suspect
			// especially when it comes to strings vs &str
			let source = self.source.clone();
			let ip_address = self.ip_address.clone();

			Server::connect_to_peer(&peer_map, source, ip_address, port.to_string());
		}
        Ok(())
    }

    /// Turns this `Announce` into a [Bing2BingFrame].
    pub fn into_frame(self) -> Bing2BingFrame {
        // note that using the vec! macro like this is more
        // performant than creating a new vector and then
        // pushing into it according to clippy:
        // https://rust-lang.github.io/rust-clippy/master/index.html#vec_init_then_push
        let mut cmd = vec![
            Bing2BingFrame::Text("announce".to_string()),
            Bing2BingFrame::Text(self.source),
            Bing2BingFrame::Number(self.sequence_number),
            Bing2BingFrame::Text(self.ip_address),
            Bing2BingFrame::Number(self.port),
            Bing2BingFrame::Number(self.available_incoming),
            Bing2BingFrame::Text(self.city),
            Bing2BingFrame::Float(self.lat),
            Bing2BingFrame::Float(self.lng),
        ];

        let mut peers = vec![];

        for (peer, time) in self.peers {
            peers.push(Bing2BingFrame::Latency(peer, time));
        }

        cmd.push(Bing2BingFrame::Array(peers));

        Bing2BingFrame::Array(cmd)
    }
}
