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
use snarkvm::prelude::{block::Block, error, Address, FromBytes, Network, TestRng, Testnet3 as CurrentNetwork};

use std::{
    collections::HashMap,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

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
use rand::Rng;
use tokio_util::codec::Framed;
use tracing::*;

const ALEO_MAXIMUM_FORK_DEPTH: u32 = 4096;

/// Loads the current network's genesis block.
pub fn sample_genesis_block() -> Block<CurrentNetwork> {
    Block::<CurrentNetwork>::from_bytes_le(CurrentNetwork::genesis_bytes()).unwrap()
}

#[derive(Clone)]
pub struct TestPeer {
    node: Node,
}

impl Pea2Pea for TestPeer {
    fn node(&self) -> &Node {
        &self.node
    }
}

impl TestPeer {
    pub async fn new() -> Self {
        let peer = Self {
            node: Node::new(Config {
                listener_ip: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                max_connections: 200,
                ..Default::default()
            }),
        };

        // peer.enable_handshake().await;
        // peer.enable_reading().await;
        // peer.enable_writing().await;
        peer.enable_disconnect().await;
        peer.node().start_listening().await.unwrap();

        peer
    }
}

// #[async_trait::async_trait]
// impl Writing for TestPeer {
//     type Codec = NoiseCodec<CurrentNetwork>;
//     type Message = EventOrBytes<CurrentNetwork>;
//
//     fn codec(&self, _addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
//         let state = self.noise_states.write().remove(&addr).unwrap();
//         NoiseCodec::new(state)
//     }
// }
//
// #[async_trait::async_trait]
// impl Reading for TestPeer {
//     type Codec = NoiseCodec<CurrentNetwork>;
//     type Message = EventOrBytes<CurrentNetwork>;
//
//     fn codec(&self, _peer_addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
//         NoiseCodec::new(state)
//     }
//
//     async fn process_message(&self, _peer_ip: SocketAddr, _message: Self::Message) -> io::Result<()> {
//         Ok(())
//     }
// }

#[async_trait::async_trait]
impl Disconnect for TestPeer {
    async fn handle_disconnect(&self, _peer_addr: SocketAddr) {}
}
