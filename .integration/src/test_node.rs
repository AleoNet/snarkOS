// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use snarkos_environment::{
    helpers::{BlockLocators, NodeType, Status},
    network::{Data, MessageCodec},
    Client,
    CurrentNetwork,
    Environment,
};
use snarkos_synthetic_node::{ClientMessage, ClientState, SynthNode, MAXIMUM_FORK_DEPTH, MESSAGE_VERSION};
use snarkvm::traits::Network;

use pea2pea::{
    protocols::{Disconnect, Handshake, Reading, Writing},
    Config,
    ConnectionSide,
    Node as Pea2PeaNode,
    Pea2Pea,
};
use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    ops::Deref,
    time::Duration,
};
use tokio::task;
use tracing::*;

// Consts & aliases.
const PING_INTERVAL_SECS: u64 = 5;
const PEER_INTERVAL_SECS: u64 = 3;
const DESIRED_CONNECTIONS: usize = <Client<CurrentNetwork>>::MINIMUM_NUMBER_OF_PEERS * 3;

pub const MAXIMUM_NUMBER_OF_PEERS: usize = <Client<CurrentNetwork>>::MAXIMUM_NUMBER_OF_PEERS;

/// The test node; it contains a `SynthNode` with some custom behavior.
#[derive(Clone)]
pub struct TestNode(SynthNode);

impl Pea2Pea for TestNode {
    fn node(&self) -> &Pea2PeaNode {
        self.0.node()
    }
}

impl Deref for TestNode {
    type Target = SynthNode;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TestNode {
    /// Creates a default test node with the most basic network protocols enabled.
    pub async fn default() -> Self {
        let config = Config {
            listener_ip: Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
            max_connections: MAXIMUM_NUMBER_OF_PEERS as u16,
            ..Default::default()
        };

        let pea2pea_node = Pea2PeaNode::new(config).await.unwrap();
        let client_state = Default::default();
        let node = TestNode::new(pea2pea_node, client_state);
        node.enable_disconnect().await;
        node.enable_handshake().await;
        node.enable_reading().await;
        node.enable_writing().await;
        node
    }

    /// Creates a test node using the given `Pea2Pea` node.
    pub fn new(node: Pea2PeaNode, state: ClientState) -> Self {
        Self(SynthNode::new(node, state))
    }

    /// Spawns a task dedicated to broadcasting Ping messages.
    pub fn send_pings(&self) {
        let node = self.clone();
        task::spawn(async move {
            let genesis = CurrentNetwork::genesis_block();
            let ping_msg = ClientMessage::Ping(
                MESSAGE_VERSION,
                MAXIMUM_FORK_DEPTH,
                NodeType::Client,
                Status::Ready,
                genesis.hash(),
                Data::Object(genesis.header().clone()),
            );

            loop {
                if node.node().num_connected() != 0 {
                    info!(parent: node.node().span(), "sending out Pings");
                    node.broadcast(ping_msg.clone()).unwrap();
                }
                tokio::time::sleep(Duration::from_secs(PING_INTERVAL_SECS)).await;
            }
        });
    }

    /// Spawns a task dedicated to peer maintenance.
    pub fn update_peers(&self) {
        let node = self.clone();
        task::spawn(async move {
            loop {
                let num_connections = node.node().num_connected() + node.node().num_connecting();
                if num_connections < DESIRED_CONNECTIONS && node.node().num_connected() != 0 {
                    info!(parent: node.node().span(), "I'd like to have {} more peers; asking peers for their peers", DESIRED_CONNECTIONS - num_connections);
                    node.broadcast(ClientMessage::PeerRequest).unwrap();
                }
                tokio::time::sleep(Duration::from_secs(PEER_INTERVAL_SECS)).await;
            }
        });
    }

    /// Starts the usual periodic activities of a test node.
    pub fn run_periodic_tasks(&self) {
        self.send_pings();
        self.update_peers();
    }
}

/// Inbound message processing logic for the test nodes.
#[async_trait::async_trait]
impl Reading for TestNode {
    type Codec = MessageCodec<CurrentNetwork, Client<CurrentNetwork>>;
    type Message = ClientMessage;

    fn codec(&self, _addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }

    async fn process_message(&self, source: SocketAddr, message: Self::Message) -> io::Result<()> {
        match message {
            ClientMessage::BlockRequest(_start_block_height, _end_block_height) => {}
            ClientMessage::BlockResponse(_block) => {}
            ClientMessage::Disconnect(reason) => {
                debug!("Peer {} disconnected for the following reason: {:?}", source, reason);
            }
            ClientMessage::PeerRequest => self.process_peer_request(source).await?,
            ClientMessage::PeerResponse(peer_addrs, _) => self.process_peer_response(source, peer_addrs).await?,
            ClientMessage::Ping(version, _fork_depth, _peer_type, _peer_state, _block_hash, block_header) => {
                // Deserialise the block header.
                let block_header = block_header.deserialize().await.unwrap();
                self.process_ping(source, version, block_header.height()).await?
            }
            ClientMessage::Pong(_is_fork, _block_locators) => {}
            ClientMessage::UnconfirmedBlock(_block_height, _block_hash, _block) => {}
            ClientMessage::UnconfirmedTransaction(_transaction) => {}
            ClientMessage::PoolRegister(_address) => {}
            ClientMessage::PoolRequest(_share_difficulty, _block_template) => {}
            ClientMessage::PoolResponse(_address, _nonce, _proof) => {}
            _ => return Err(io::ErrorKind::InvalidData.into()), // Peer is not following the protocol.
        }

        Ok(())
    }
}

// Helper methods.
impl TestNode {
    async fn process_peer_request(&self, source: SocketAddr) -> io::Result<()> {
        let peers = self.state.peers.read().keys().copied().collect::<Vec<_>>();
        let msg = ClientMessage::PeerResponse(peers, None);
        info!(parent: self.node().span(), "sending a PeerResponse to {}", source);

        self.unicast(source, msg)?;

        Ok(())
    }

    async fn process_peer_response(&self, source: SocketAddr, peer_addrs: Vec<SocketAddr>) -> io::Result<()> {
        let num_connections = self.node().num_connected() + self.node().num_connecting();
        let node = self.clone();
        task::spawn(async move {
            for peer_addr in peer_addrs
                .into_iter()
                .filter(|addr| node.node().listening_addr().unwrap() != *addr)
                .take(DESIRED_CONNECTIONS.saturating_sub(num_connections))
            {
                if !node.node().is_connected(peer_addr) && !node.state.peers.read().contains_key(&peer_addr) {
                    info!(parent: node.node().span(), "trying to connect to {}'s peer {}", source, peer_addr);
                    let _ = node.node().connect(peer_addr).await;
                }
            }
        });

        Ok(())
    }

    async fn process_ping(&self, source: SocketAddr, version: u32, block_height: u32) -> io::Result<()> {
        // Ensure the message protocol version is not outdated.
        if version < <Client<CurrentNetwork>>::MESSAGE_VERSION {
            warn!(parent: self.node().span(), "dropping {} due to outdated version ({})", source, version);
            return Err(io::ErrorKind::InvalidData.into());
        }

        debug!(parent: self.node().span(), "peer {} is at height {}", source, block_height);

        let genesis = CurrentNetwork::genesis_block();
        let msg = ClientMessage::Pong(
            None,
            Data::Object(
                BlockLocators::<CurrentNetwork>::from(vec![(genesis.height(), (genesis.hash(), None))].into_iter().collect()).unwrap(),
            ),
        );

        info!(parent: self.node().span(), "sending a Pong to {}", source);

        self.unicast(source, msg)?;

        Ok(())
    }
}
