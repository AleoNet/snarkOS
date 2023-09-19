// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use snarkos_node_narwhal::helpers::{EventOrBytes, NoiseCodec, NoiseState};
use snarkvm::prelude::Testnet3 as CurrentNetwork;

use std::{
    collections::HashMap,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use bytes::Bytes;
use futures_util::{sink::SinkExt, TryStreamExt};
use parking_lot::RwLock;
use pea2pea::{
    protocols::{Disconnect, Handshake, Reading, Writing},
    Config,
    Connection,
    ConnectionSide,
    Node,
    Pea2Pea,
};

use snow::{params::NoiseParams, Builder};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    time::timeout,
};
use tokio_util::codec::{Framed, FramedParts};

pub struct TestPeer {
    inner_node: InnerNode,
    inbound_rx: Receiver<(SocketAddr, EventOrBytes<CurrentNetwork>)>,
}

#[derive(Clone)]
struct InnerNode {
    // The pea2pea node instance.
    node: Node,
    // The noise states for each connection.
    noise_states: Arc<RwLock<HashMap<SocketAddr, NoiseState>>>,
    // The inbound channel sender, used to consolidate inbound messages into a single queue so they
    // can be read in order in tests.
    inbound_tx: Sender<(SocketAddr, EventOrBytes<CurrentNetwork>)>,
}

impl TestPeer {
    pub async fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        let inner_node = InnerNode {
            node: Node::new(Config {
                listener_ip: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                max_connections: 200,
                ..Default::default()
            }),
            noise_states: Default::default(),
            inbound_tx: tx,
        };

        inner_node.enable_handshake().await;
        inner_node.enable_reading().await;
        inner_node.enable_writing().await;
        inner_node.enable_disconnect().await;
        inner_node.node().start_listening().await.unwrap();

        Self { inner_node, inbound_rx: rx }
    }

    pub fn listening_addr(&self) -> SocketAddr {
        self.inner_node.node().listening_addr().expect("addr should be present")
    }

    pub async fn connect(&self, target: SocketAddr) -> io::Result<()> {
        self.inner_node.node().connect(target).await?;
        Ok(())
    }

    // Note: the codec doesn't actually support sending bytes post-handshake, perhaps this should
    // be relaxed by making a test-only codec in future.
    pub fn unicast(&self, target: SocketAddr, message: EventOrBytes<CurrentNetwork>) -> io::Result<()> {
        self.inner_node.unicast(target, message)?;
        Ok(())
    }

    pub async fn recv(&mut self) -> (SocketAddr, EventOrBytes<CurrentNetwork>) {
        match self.inbound_rx.recv().await {
            Some(message) => message,
            None => panic!("all senders dropped!"),
        }
    }

    pub async fn recv_timeout(&mut self, duration: Duration) -> (SocketAddr, EventOrBytes<CurrentNetwork>) {
        match timeout(duration, self.recv()).await {
            Ok(message) => message,
            _ => panic!("timed out waiting for message"),
        }
    }
}

impl Pea2Pea for InnerNode {
    fn node(&self) -> &Node {
        &self.node
    }
}

const NOISE_HANDSHAKE_TYPE: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

#[async_trait::async_trait]
impl Handshake for InnerNode {
    // Set the timeout on the test peer to be longer than the gateway's timeout.
    const TIMEOUT_MS: u64 = 10_000;

    async fn perform_handshake(&self, mut connection: Connection) -> io::Result<Connection> {
        let stream = self.borrow_stream(&mut connection);

        // Set up the noise codec.
        let params: NoiseParams = NOISE_HANDSHAKE_TYPE.parse().unwrap();
        let noise_builder = Builder::new(params);
        let kp = noise_builder.generate_keypair().unwrap();
        let initiator = noise_builder.local_private_key(&kp.private).build_initiator().unwrap();
        let codec = NoiseCodec::<CurrentNetwork>::new(NoiseState::Handshake(Box::new(initiator)));

        // Construct the stream.
        let mut framed = Framed::new(stream, codec);

        /* Noise handshake */

        // -> e
        framed.send(EventOrBytes::Bytes(Bytes::new())).await?;

        // <- e, ee, s, es
        framed.try_next().await?;

        // -> s, se
        framed.send(EventOrBytes::Bytes(Bytes::new())).await?;

        //  // Set the codec to post-handshake mode.
        //  let mut framed =
        //      framed.map_codec(|c| NoiseCodec::<CurrentNetwork>::new(c.noise_state.into_post_handshake_state()));

        let FramedParts { codec, .. } = framed.into_parts();
        let NoiseCodec { noise_state, .. } = codec;

        self.noise_states.write().insert(connection.addr(), noise_state.into_post_handshake_state());

        Ok(connection)
    }
}

#[async_trait::async_trait]
impl Writing for InnerNode {
    type Codec = NoiseCodec<CurrentNetwork>;
    type Message = EventOrBytes<CurrentNetwork>;

    fn codec(&self, peer_addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        let state = self.noise_states.write().remove(&peer_addr).unwrap();
        NoiseCodec::new(state)
    }
}

#[async_trait::async_trait]
impl Reading for InnerNode {
    type Codec = NoiseCodec<CurrentNetwork>;
    type Message = EventOrBytes<CurrentNetwork>;

    fn codec(&self, peer_addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        let state = self.noise_states.read().get(&peer_addr).unwrap().clone();
        NoiseCodec::new(state)
    }

    async fn process_message(&self, peer_addr: SocketAddr, message: Self::Message) -> io::Result<()> {
        self.inbound_tx.send((peer_addr, message)).await.map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "failed to send message to test peer, all receivers have been dropped")
        })
    }
}

#[async_trait::async_trait]
impl Disconnect for InnerNode {
    async fn handle_disconnect(&self, _peer_addr: SocketAddr) {}
}
