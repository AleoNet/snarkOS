// Copyright (C) 2019-2020 Aleo Systems Inc.
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

// Compilation
#![allow(clippy::module_inception)]
#![warn(unused_extern_crates)]
#![forbid(unsafe_code)]
// Documentation
#![cfg_attr(nightly, feature(doc_cfg, external_doc))]
#![cfg_attr(nightly, doc(include = "../documentation/concepts/network_server.md"))]

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate snarkos_metrics;

pub mod external;

pub mod blocks;
pub use blocks::*;

pub mod environment;
pub use environment::*;

pub mod errors;
pub use errors::*;

pub mod inbound;
pub use inbound::*;

pub mod outbound;
pub use outbound::*;

pub mod peers;
pub use peers::*;

use crate::{external::Channel, peers::peers::Peers};

use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{sync::RwLock, task, time::sleep};

pub(crate) type Sender = tokio::sync::mpsc::Sender<Response>;

pub(crate) type Receiver = tokio::sync::mpsc::Receiver<Response>;

/// A core data structure for operating the networking stack of this node.
#[derive(Clone)]
pub struct Server {
    /// The parameters and settings of this node server.
    environment: Environment,
    /// The inbound handler of this node server.
    inbound: Arc<Inbound>,
    /// The outbound handler of this node server.
    outbound: Arc<Outbound>,

    peers: Peers,
    blocks: Blocks,
    // sync_manager: Arc<Mutex<SyncManager>>,
}

impl Server {
    /// Creates a new instance of `Server`.
    pub async fn new(environment: Environment) -> Result<Self, NetworkError> {
        let channels: Arc<RwLock<HashMap<SocketAddr, Channel>>> = Default::default();
        // Create the inbound and outbound handlers.
        let inbound = Arc::new(Inbound::new(channels.clone()));
        let outbound = Arc::new(Outbound::new(channels));

        // Initialize the peer and block services.
        let peers = Peers::new(environment.clone(), inbound.clone(), outbound.clone())?;
        let blocks = Blocks::new(environment.clone(), outbound.clone())?;

        Ok(Self {
            environment,
            inbound,
            outbound,
            peers,
            blocks,
        })
    }

    #[inline]
    pub async fn start(&mut self) -> Result<(), NetworkError> {
        debug!("Initializing server");
        self.inbound.listen(&mut self.environment).await?;

        // update the local address for Blocks and Peers
        self.peers
            .environment
            .set_local_address(self.environment.local_address().unwrap());
        self.blocks
            .environment
            .set_local_address(self.environment.local_address().unwrap());

        let peers = self.peers.clone();
        let blocks = self.blocks.clone();
        task::spawn(async move {
            loop {
                info!("Updating peers and blocks");
                if let Err(e) = peers.update().await {
                    error!("Peer update error: {}", e);
                }
                if let Err(e) = blocks.update().await {
                    error!("Block update error: {}", e);
                }
                sleep(Duration::from_secs(10)).await;
            }
        });

        let server_clone = self.clone();
        task::spawn(async move {
            loop {
                if let Err(e) = server_clone.receive_response().await {
                    error!("Server error: {}", e);
                }
            }
        });

        debug!("Initialized server");
        Ok(())
    }

    #[inline]
    pub fn local_address(&self) -> Option<SocketAddr> {
        self.environment.local_address()
    }

    async fn receive_response(&self) -> Result<(), NetworkError> {
        let response = self
            .inbound
            .receiver()
            .lock()
            .await
            .recv()
            .await
            .ok_or(NetworkError::ReceiverFailedToParse)?;

        match response {
            Response::ConnectingTo(remote_address, nonce) => {
                self.peers.connecting_to_peer(remote_address, nonce).await?;
            }
            Response::ConnectedTo(remote_address, nonce) => {
                self.peers.connected_to_peer(remote_address, nonce).await?;
            }
            Response::VersionToVerack(remote_address, remote_version) => {
                self.peers.version_to_verack(remote_address, &remote_version).await?;
            }
            Response::Verack(remote_address, verack) => {
                self.peers.verack(&remote_address, &verack).await?;
            }
            Response::Transaction(source, transaction) => {
                let connected_peers = self.peers.connected_peers().await;
                self.blocks
                    .received_transaction(source, transaction, connected_peers)
                    .await?;
            }
            Response::Block(remote_address, block, propagate) => {
                let connected_peers = match propagate {
                    true => Some(self.peers.connected_peers().await),
                    false => None,
                };
                self.blocks
                    .received_block(remote_address, block, connected_peers)
                    .await?;
            }
            Response::GetBlock(remote_address, getblock) => {
                self.blocks.received_get_block(remote_address, getblock).await?;
            }
            Response::GetMemoryPool(remote_address) => {
                self.blocks.received_get_memory_pool(remote_address).await?;
            }
            Response::MemoryPool(mempool) => {
                self.blocks.received_memory_pool(mempool).await?;
            }
            Response::GetSync(remote_address, getsync) => {
                self.blocks.received_get_sync(remote_address, getsync).await?;
            }
            Response::Sync(remote_address, sync) => {
                self.blocks.received_sync(sync).await?;
            }
            Response::DisconnectFrom(remote_address) => {
                self.peers.disconnected_from_peer(&remote_address).await?;
            }
            Response::GetPeers(remote_address) => {
                self.peers.send_get_peers(remote_address).await?;
            }
            Response::Peers(remote_address, peers) => {
                self.peers.inbound_peers(remote_address, peers).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::{
        channel::Channel,
        message::{read_header, read_message, Message, MessageHeader},
        Verack,
        Version,
    };

    use snarkos_consensus::MemoryPool;
    use snarkos_testing::{
        consensus::{FIXTURE_VK, TEST_CONSENSUS},
        dpc::load_verifying_parameters,
    };

    use std::{sync::Arc, time::Duration};

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        sync::{Mutex, RwLock},
    };

    async fn test_node(bootnodes: Vec<String>) -> Server {
        let storage = FIXTURE_VK.ledger();
        let memory_pool = MemoryPool::new();
        let memory_pool_lock = Arc::new(Mutex::new(memory_pool));
        let consensus = TEST_CONSENSUS.clone();
        let parameters = load_verifying_parameters();
        let socket_address = None;
        let min_peers = 1;
        let max_peers = 10;
        let sync_interval = 100;
        let mempool_interval = 5;
        let is_bootnode = false;
        let is_miner = false;

        let environment = Environment::new(
            Arc::new(RwLock::new(storage)),
            memory_pool_lock,
            Arc::new(consensus),
            Arc::new(parameters),
            socket_address,
            min_peers,
            max_peers,
            sync_interval,
            mempool_interval,
            bootnodes,
            is_bootnode,
            is_miner,
        )
        .unwrap();

        Server::new(environment).await.unwrap()
    }

    #[tokio::test]
    async fn starts_server() {
        let mut server = test_node(vec![]).await;

        assert!(server.start().await.is_ok());
        assert_eq!(server.peers.number_of_connected_peers().await, 0);
    }

    #[tokio::test]
    async fn handshake_responder_side() {
        // start a test node and listen for incoming connections
        let mut node = test_node(vec![]).await;
        node.start().await.unwrap();
        let node_listener = node.local_address().unwrap();

        // set up a fake node (peer), which is just a socket
        let mut peer_stream = TcpStream::connect(&node_listener).await.unwrap();

        // register the addresses bound to the connection between the node and the peer
        let peer_address = peer_stream.local_addr().unwrap();
        let node_address = peer_stream.peer_addr().unwrap();

        // the peer initiates a handshake by sending a Version message
        let version = Version::new(1u64, 1u32, 1u64, peer_address, node_address);
        let serialized = version.serialize().unwrap();
        let header = MessageHeader::new(Version::name(), serialized.len() as u32)
            .serialize()
            .unwrap();
        peer_stream.write_all(&header).await.unwrap();
        peer_stream.write_all(&serialized).await.unwrap();
        peer_stream.flush().await.unwrap();

        // at this point the node should have marked the peer as 'connecting'
        sleep(Duration::from_millis(200)).await;
        assert!(node.peers.is_connecting(&peer_address).await);

        // check if the peer has received the Verack message from the node
        let header = read_header(&mut peer_stream).await.unwrap();
        let message = read_message(&mut peer_stream, header.len as usize).await.unwrap();
        let _verack = Verack::deserialize(&message).unwrap();

        // check if it was followed by a Version message
        let header = read_header(&mut peer_stream).await.unwrap();
        let message = read_message(&mut peer_stream, header.len as usize).await.unwrap();
        let version = Version::deserialize(&message).unwrap();

        // in response to the Version, the peer sends a Verack message to finish the handshake
        let verack = Verack::new(version.nonce, peer_address, node_address);
        let serialized = verack.serialize().unwrap();
        let header = MessageHeader::new(Verack::name(), serialized.len() as u32)
            .serialize()
            .unwrap();
        peer_stream.write_all(&header).await.unwrap();
        peer_stream.write_all(&serialized).await.unwrap();
        peer_stream.flush().await.unwrap();

        // the node should now have register the peer as 'connected'
        sleep(Duration::from_millis(200)).await;
        assert!(node.peers.is_connected(&peer_address).await);
        assert_eq!(node.peers.number_of_connected_peers().await, 1);
    }

    #[tokio::test]
    async fn handshake_initiator_side() {
        // start a fake peer which is just a socket
        let peer_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let peer_address = peer_listener.local_addr().unwrap();

        // start node with the peer as a bootnode; that way it will get connected to
        let mut node = test_node(vec![peer_address.to_string()]).await;
        node.start().await.unwrap();

        // accept the node's connection on peer side
        let (mut peer_stream, node_address) = peer_listener.accept().await.unwrap();

        // the peer should receive a Version message from the node (initiator of the handshake)
        let header = read_header(&mut peer_stream).await.unwrap();
        let message = read_message(&mut peer_stream, header.len as usize).await.unwrap();
        let version = Version::deserialize(&message).unwrap();

        // at this point the node should have marked the peer as 'connecting'
        assert!(node.peers.is_connecting(&peer_address).await);

        // the peer responds with a Verack acknowledging the Version message
        let verack = Verack::new(version.nonce, peer_address, node_address);
        let serialized = verack.serialize().unwrap();
        let header = MessageHeader::new(Verack::name(), serialized.len() as u32)
            .serialize()
            .unwrap();
        peer_stream.write_all(&header).await.unwrap();
        peer_stream.write_all(&serialized).await.unwrap();
        peer_stream.flush().await.unwrap();

        // the peer then follows up with a Version message
        let version = Version::new(1u64, 1u32, 1u64, peer_address, node_address);
        let serialized = version.serialize().unwrap();
        let header = MessageHeader::new(Version::name(), serialized.len() as u32)
            .serialize()
            .unwrap();
        peer_stream.write_all(&header).await.unwrap();
        peer_stream.write_all(&serialized).await.unwrap();
        peer_stream.flush().await.unwrap();

        // the node should now have registered the peer as 'connected'
        sleep(Duration::from_millis(200)).await;
        assert!(node.peers.is_connected(&peer_address).await);
        assert_eq!(node.peers.number_of_connected_peers().await, 1);
    }
}
