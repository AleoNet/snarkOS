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

use crate::{constants::*, known_network::KnownNetwork};
use snarkos_environment::CurrentNetwork;
use snarkos_network::Data;
use snarkos_storage::BlockLocators;
use snarkos_synthetic_node::{ClientMessage, SynthNode, MAXIMUM_FORK_DEPTH, MESSAGE_LENGTH_PREFIX_SIZE, MESSAGE_VERSION};
use snarkvm::traits::Network;

use pea2pea::{
    protocols::{Disconnect, Handshake, Reading, Writing},
    Config,
    Node as Pea2PeaNode,
    Pea2Pea,
};
use std::{convert::TryInto, io, net::SocketAddr, ops::Deref, sync::Arc, time::Duration};
use structopt::StructOpt;
use tokio::task;
use tracing::*;

// CLI
#[derive(Debug, StructOpt)]
pub struct Opts {
    /// Specify the IP address and port for the node server.
    /// Naming and defaults kept consistent with snarkOS.
    #[structopt(parse(try_from_str), default_value = "0.0.0.0:4132", long = "node")]
    pub node: SocketAddr,
}

#[derive(Clone)]
pub struct Crawler {
    synth_node: SynthNode,
    known_network: Arc<KnownNetwork>,
}

impl Pea2Pea for Crawler {
    fn node(&self) -> &Pea2PeaNode {
        &self.synth_node.node()
    }
}

impl Deref for Crawler {
    type Target = SynthNode;

    fn deref(&self) -> &Self::Target {
        &self.synth_node
    }
}

impl Crawler {
    /// Creates a crawler node with the most basic network protocols enabled.
    pub async fn new(opts: Opts) -> Self {
        let config = Config {
            listener_ip: Some(opts.node.ip()),
            desired_listening_port: Some(opts.node.port()),
            max_connections: MAXIMUM_NUMBER_OF_PEERS as u16,
            ..Default::default()
        };

        let pea2pea_node = Pea2PeaNode::new(Some(config)).await.unwrap();
        let client_state = Default::default();
        let node = Self {
            synth_node: SynthNode::new(pea2pea_node, client_state),
            known_network: Arc::new(KnownNetwork::default()),
        };

        node.enable_disconnect().await;
        node.enable_handshake().await;
        node.enable_reading().await;
        node.enable_writing().await;

        node
    }

    /// Spawns a task dedicated to broadcasting Ping messages.
    pub fn send_pings(&self) {
        let node = self.clone();
        task::spawn(async move {
            let genesis = CurrentNetwork::genesis_block();
            let ping_msg = ClientMessage::Ping(
                MESSAGE_VERSION,
                MAXIMUM_FORK_DEPTH,
                node.node_type(),
                node.state(),
                genesis.hash(),
                Data::Object(genesis.header().clone()),
            );

            loop {
                if node.node().num_connected() != 0 {
                    info!(parent: node.node().span(), "sending out Pings");
                    node.send_broadcast(ping_msg.clone()).unwrap();
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
                // Disconnect from peers we have just crawled.
                for addr in node.known_network.addrs_to_disconnect() {
                    node.node().disconnect(addr).await;
                }

                // Connect to peers we haven't crawled in a while.
                for addr in node.known_network.addrs_to_connect() {
                    if !node.node().is_connected(addr) {
                        let _ = node.node().connect(addr).await;
                    }
                }

                info!(parent: node.node().span(), "Crawling the netowrk for more peers; asking peers for their peers");
                node.send_broadcast(ClientMessage::PeerRequest).unwrap();
                tokio::time::sleep(Duration::from_secs(PEER_INTERVAL_SECS)).await;
            }
        });
    }

    fn log_known_network(&self) {
        let node = self.clone();
        tokio::spawn(async move {
            loop {
                info!(parent: node.node().span(), "Known addresses: {}", node.known_network.nodes().len());
                info!(parent: node.node().span(), "Known connections: {}", node.known_network.connections().len());
                tokio::time::sleep(Duration::from_secs(LOG_INTERVAL_SECS)).await;
            }
        });
    }

    /// Starts the usual periodic activities of a crawler node.
    pub fn run_periodic_tasks(&self) {
        self.send_pings();
        self.update_peers();
        self.log_known_network();
    }
}

/// Inbound message processing logic for the crawler nodes.
#[async_trait::async_trait]
impl Reading for Crawler {
    type Message = ClientMessage;

    fn read_message<R: io::Read>(&self, source: SocketAddr, reader: &mut R) -> io::Result<Option<Self::Message>> {
        // FIXME: use the maximum message size allowed by the protocol or (better) use streaming deserialization.
        let mut buf = [0u8; 8 * 1024];

        reader.read_exact(&mut buf[..MESSAGE_LENGTH_PREFIX_SIZE])?;
        let len = u32::from_le_bytes(buf[..MESSAGE_LENGTH_PREFIX_SIZE].try_into().unwrap()) as usize;

        if reader.read_exact(&mut buf[..len]).is_err() {
            return Ok(None);
        }

        match ClientMessage::deserialize(&mut io::Cursor::new(&buf[..len])) {
            Ok(msg) => {
                info!(parent: self.node().span(), "received a {} from {}", msg.name(), source);
                Ok(Some(msg))
            }
            Err(e) => {
                error!("a message from {} failed to deserialize: {}", source, e);
                Err(io::ErrorKind::InvalidData.into())
            }
        }
    }

    async fn process_message(&self, source: SocketAddr, message: Self::Message) -> io::Result<()> {
        match message {
            ClientMessage::BlockRequest(_start_block_height, _end_block_height) => {}
            ClientMessage::BlockResponse(_block) => {}
            ClientMessage::Disconnect(reason) => {
                debug!("Peer {} disconnected for the following reason: {:?}", source, reason);
            }
            ClientMessage::PeerRequest => self.process_peer_request(source).await?,
            ClientMessage::PeerResponse(peer_ips) => self.process_peer_response(source, peer_ips).await?,
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
impl Crawler {
    async fn process_peer_request(&self, source: SocketAddr) -> io::Result<()> {
        let peers = vec![];
        let msg = ClientMessage::PeerResponse(peers);
        info!(parent: self.node().span(), "sending a PeerResponse to {}", source);

        self.send_direct_message(source, msg)?;

        Ok(())
    }

    async fn process_peer_response(&self, source: SocketAddr, mut peer_ips: Vec<SocketAddr>) -> io::Result<()> {
        let node = self.clone();
        task::spawn(async move {
            peer_ips.retain(|addr| node.node().listening_addr().unwrap() != *addr);

            // Insert the address into the known network and update the crawl state.
            node.known_network.update_connections(source, peer_ips.clone());
            node.known_network.received_peers(source);

            for peer_ip in peer_ips {
                if !node.node().is_connected(peer_ip) && !node.state.peers.lock().await.iter().any(|peer| peer.listening_addr == peer_ip) {
                    info!(parent: node.node().span(), "trying to connect to {}'s peer {}", source, peer_ip);

                    // Only connect if this address is unknown.
                    if !node.known_network.nodes().contains_key(&peer_ip) {
                        let _ = node.node().connect(peer_ip).await;
                    }
                }
            }
        });

        Ok(())
    }

    async fn process_ping(&self, source: SocketAddr, version: u32, block_height: u32) -> io::Result<()> {
        // Ensure the message protocol version is not outdated.
        if version < MESSAGE_VERSION {
            warn!(parent: self.node().span(), "dropping {} due to outdated version ({})", source, version);
            return Err(io::ErrorKind::InvalidData.into());
        }

        debug!(parent: self.node().span(), "peer {} is at height {}", source, block_height);

        // Update the known network nodes and update the crawl state.
        self.known_network.update_height(source, block_height);
        self.known_network.received_height(source);

        let genesis = CurrentNetwork::genesis_block();
        let msg = ClientMessage::Pong(
            None,
            Data::Object(
                BlockLocators::<CurrentNetwork>::from(vec![(genesis.height(), (genesis.hash(), None))].into_iter().collect()).unwrap(),
            ),
        );

        info!(parent: self.node().span(), "sending a Pong to {}", source);

        self.send_direct_message(source, msg)?;

        Ok(())
    }
}
