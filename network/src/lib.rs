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
extern crate thiserror;

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

pub mod transactions;
pub use transactions::*;

use crate::{external::message::*, peers::peers::Peers, ConnWriter};

use parking_lot::RwLock;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{task, time::sleep};

pub(crate) type Sender = tokio::sync::mpsc::Sender<Message>;

pub(crate) type Receiver = tokio::sync::mpsc::Receiver<Message>;

/// A core data structure for operating the networking stack of this node.
#[derive(Clone)]
pub struct Server {
    /// The parameters and settings of this node server.
    pub environment: Environment,
    /// The inbound handler of this node server.
    inbound: Arc<Inbound>,
    /// The outbound handler of this node server.
    outbound: Arc<Outbound>,

    pub peers: Peers,
    pub blocks: Blocks,
    pub transactions: Transactions,
}

impl Server {
    /// Creates a new instance of `Server`.
    pub async fn new(environment: Environment) -> Result<Self, NetworkError> {
        let channels: Arc<RwLock<HashMap<SocketAddr, ConnWriter>>> = Default::default();
        // Create the inbound and outbound handlers.
        let inbound = Arc::new(Inbound::new(channels.clone()));
        let outbound = Arc::new(Outbound::new(channels));

        // Initialize the peer and block services.
        let peers = Peers::new(environment.clone(), inbound.clone(), outbound.clone())?;
        let blocks = Blocks::new(environment.clone(), outbound.clone());
        let transactions = Transactions::new(environment.clone(), outbound.clone());

        Ok(Self {
            environment,
            inbound,
            outbound,
            peers,
            blocks,
            transactions,
        })
    }

    pub async fn establish_address(&mut self) -> Result<(), NetworkError> {
        self.inbound.listen(&mut self.environment).await?;
        let address = self.environment.local_address().unwrap();

        // update the local address for Blocks and Peers
        self.peers.environment.set_local_address(address);
        self.blocks.environment.set_local_address(address);

        Ok(())
    }

    pub async fn start_services(&self) {
        let peer_sync_interval = self.environment.peer_sync_interval();
        let peers = self.peers.clone();

        let block_sync_interval = self.environment.block_sync_interval();
        let blocks = self.blocks.clone();

        let transaction_sync_interval = self.environment.transaction_sync_interval();
        let transactions = self.transactions.clone();

        task::spawn(async move {
            loop {
                sleep(peer_sync_interval).await;
                info!("Updating peers");

                if let Err(e) = peers.update().await {
                    error!("Peer update error: {}", e);
                }
            }
        });

        let peers = self.peers.clone();
        task::spawn(async move {
            loop {
                sleep(block_sync_interval).await;
                info!("Updating blocks");

                // select last seen node as block sync node
                let sync_node = peers.last_seen();

                blocks.update(sync_node).await;
            }
        });

        let peers = self.peers.clone();
        task::spawn(async move {
            loop {
                sleep(transaction_sync_interval).await;
                info!("Updating transactions");

                // select last seen node as block sync node
                let sync_node = peers.last_seen();

                if let Err(e) = transactions.update(sync_node) {
                    error!("Transaction update error: {}", e);
                }
            }
        });

        let server = self.clone();
        let mut receiver = self.inbound.take_receiver();
        task::spawn(async move {
            loop {
                if let Err(e) = server.receive_response(&mut receiver).await {
                    error!("Server error: {}", e);
                }
            }
        });
    }

    pub async fn start(&mut self) -> Result<(), NetworkError> {
        debug!("Initializing the connection server");
        self.establish_address().await?;
        self.start_services().await;
        debug!("Connection server initialized");

        Ok(())
    }

    #[inline]
    pub fn local_address(&self) -> Option<SocketAddr> {
        self.environment.local_address()
    }

    async fn receive_response(&self, receiver: &mut Receiver) -> Result<(), NetworkError> {
        let Message { direction, payload } = receiver.recv().await.ok_or(NetworkError::ReceiverFailedToParse)?;

        let source = if let Direction::Inbound(addr) = direction {
            Some(addr)
        } else {
            None
        };

        match payload {
            Payload::ConnectingTo(remote_address, nonce) => {
                if direction == Direction::Internal {
                    self.peers.connecting_to_peer(remote_address, nonce)?;
                }
            }
            Payload::ConnectedTo(remote_address, nonce) => {
                if direction == Direction::Internal {
                    self.peers.connected_to_peer(remote_address, nonce)?;
                }
            }
            Payload::Version(version) => {
                self.peers.version_to_verack(source.unwrap(), &version)?;
            }
            Payload::Verack(verack) => {
                self.peers.verack(&verack);
            }
            Payload::Transaction(transaction) => {
                let connected_peers = self.peers.connected_peers();
                self.transactions
                    .received_transaction(source.unwrap(), transaction, connected_peers)
                    .await?;
            }
            Payload::Block(block) => {
                self.blocks
                    .received_block(source.unwrap(), block, Some(self.peers.connected_peers()))
                    .await?;
            }
            Payload::SyncBlock(block) => {
                self.blocks.received_block(source.unwrap(), block, None).await?;
            }
            Payload::GetBlock(hash) => {
                self.blocks.received_get_block(source.unwrap(), hash).await?;
            }
            Payload::GetMemoryPool => {
                self.transactions.received_get_memory_pool(source.unwrap()).await?;
            }
            Payload::MemoryPool(mempool) => {
                self.transactions.received_memory_pool(mempool)?;
            }
            Payload::GetSync(getsync) => {
                self.blocks.received_get_sync(source.unwrap(), getsync).await?;
            }
            Payload::Sync(sync) => {
                self.blocks.received_sync(source.unwrap(), sync).await;
            }
            Payload::Disconnect(addr) => {
                if direction == Direction::Internal {
                    self.peers.disconnected_from_peer(&addr)?;
                }
            }
            Payload::GetPeers => {
                self.peers.send_get_peers(source.unwrap());
            }
            Payload::Peers(peers) => {
                self.peers.process_inbound_peers(peers)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::message_types::*;
    use snarkos_testing::{
        consensus::{BLOCK_1, BLOCK_1_HEADER_HASH, BLOCK_2, BLOCK_2_HEADER_HASH, FIXTURE_VK, TEST_CONSENSUS},
        dpc::load_verifying_parameters,
        network::{read_header, read_payload},
    };
    use snarkvm_objects::block_header_hash::BlockHeaderHash;

    use std::{sync::Arc, time::Duration};

    use chrono::Utc;
    use parking_lot::{Mutex, RwLock};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    async fn test_node(
        bootnodes: Vec<String>,
        peer_sync_interval: Duration,
        block_sync_interval: Duration,
        transaction_sync_interval: Duration,
    ) -> Server {
        let storage = FIXTURE_VK.ledger();
        let memory_pool = snarkos_consensus::MemoryPool::new();
        let memory_pool_lock = Arc::new(Mutex::new(memory_pool));
        let consensus = TEST_CONSENSUS.clone();
        let parameters = load_verifying_parameters();
        let socket_address = None;
        let min_peers = 1;
        let max_peers = 10;
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
            bootnodes,
            is_bootnode,
            is_miner,
            peer_sync_interval,
            block_sync_interval,
            transaction_sync_interval,
        )
        .unwrap();

        Server::new(environment).await.unwrap()
    }

    async fn write_message_to_stream(payload: Payload, peer_stream: &mut TcpStream) {
        let payload = bincode::serialize(&payload).unwrap();
        let header = MessageHeader {
            len: payload.len() as u32,
        }
        .as_bytes();
        peer_stream.write_all(&header[..]).await.unwrap();
        peer_stream.write_all(&payload).await.unwrap();
        peer_stream.flush().await.unwrap();
    }

    #[tokio::test]
    async fn starts_server() {
        let mut server = test_node(vec![], Duration::new(10, 0), Duration::new(10, 0), Duration::new(10, 0)).await;
        assert!(server.start().await.is_ok());
        let address = server.local_address().unwrap();

        assert!(TcpListener::bind(address).await.is_err());
        assert_eq!(server.peers.number_of_connected_peers(), 0);
    }

    #[tokio::test]
    async fn handshake_responder_side() {
        // start a test node and listen for incoming connections
        let mut node = test_node(vec![], Duration::new(10, 0), Duration::new(10, 0), Duration::new(10, 0)).await;
        node.start().await.unwrap();
        let node_listener = node.local_address().unwrap();

        // set up a fake node (peer), which is just a socket
        let mut peer_stream = TcpStream::connect(&node_listener).await.unwrap();

        // register the addresses bound to the connection between the node and the peer
        let peer_address = peer_stream.local_addr().unwrap();

        // the peer initiates a handshake by sending a Version message
        let version = Payload::Version(Version::new(1u64, 1u32, 1u64, peer_address.port()));
        write_message_to_stream(version, &mut peer_stream).await;

        // at this point the node should have marked the peer as ' connecting'
        sleep(Duration::from_millis(200)).await;
        assert!(node.peers.is_connecting(&peer_address));

        // the buffer for peer's reads
        let mut peer_buf = [0u8; 64];

        // check if the peer has received the Verack message from the node
        let len = read_header(&mut peer_stream).await.unwrap().len();
        let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
        assert!(matches!(bincode::deserialize(&payload).unwrap(), Payload::Verack(..)));

        // check if it was followed by a Version message
        let len = read_header(&mut peer_stream).await.unwrap().len();
        let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
        let version = if let Payload::Version(version) = bincode::deserialize(&payload).unwrap() {
            version
        } else {
            unreachable!();
        };

        // in response to the Version, the peer sends a Verack message to finish the handshake
        let verack = Payload::Verack(Verack::new(version.nonce));
        write_message_to_stream(verack, &mut peer_stream).await;

        // the node should now have register the peer as 'connected'
        sleep(Duration::from_millis(200)).await;
        assert!(node.peers.is_connected(&peer_address));
        assert_eq!(node.peers.number_of_connected_peers(), 1);
    }

    #[tokio::test]
    async fn handshake_initiator_side() {
        // start a fake peer which is just a socket
        let peer_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let peer_address = peer_listener.local_addr().unwrap();

        // start node with the peer as a bootnode; that way it will get connected to
        let mut node = test_node(
            vec![peer_address.to_string()],
            Duration::new(10, 0),
            Duration::new(10, 0),
            Duration::new(10, 0),
        )
        .await;
        node.start().await.unwrap();

        // accept the node's connection on peer side
        let (mut peer_stream, _node_address) = peer_listener.accept().await.unwrap();

        // the buffer for peer's reads
        let mut peer_buf = [0u8; 64];

        // the peer should receive a Version message from the node (initiator of the handshake)
        let len = read_header(&mut peer_stream).await.unwrap().len();
        let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
        let version = if let Payload::Version(version) = bincode::deserialize(&payload).unwrap() {
            version
        } else {
            unreachable!();
        };

        // at this point the node should have marked the peer as 'connecting'
        assert!(node.peers.is_connecting(&peer_address));

        // the peer responds with a Verack acknowledging the Version message
        let verack = Payload::Verack(Verack::new(version.nonce));
        write_message_to_stream(verack, &mut peer_stream).await;

        // the peer then follows up with a Version message
        let version = Payload::Version(Version::new(1u64, 1u32, 1u64, peer_address.port()));
        write_message_to_stream(version, &mut peer_stream).await;

        // the node should now have registered the peer as 'connected'
        sleep(Duration::from_millis(200)).await;
        assert!(node.peers.is_connected(&peer_address));
        assert_eq!(node.peers.number_of_connected_peers(), 1);
    }

    async fn assert_node_rejected_message(node: &Server, peer_stream: &mut TcpStream) {
        // slight delay for server to process the message
        sleep(Duration::from_millis(200)).await;

        // read the response from the stream
        let mut buffer = String::new();
        let bytes_read = peer_stream.read_to_string(&mut buffer).await.unwrap();

        // check node's response is empty
        assert_eq!(bytes_read, 0);
        assert!(buffer.is_empty());

        // check the node's state hasn't been altered by the message
        assert!(!node.peers.is_connecting(&peer_stream.local_addr().unwrap()));
        assert_eq!(node.peers.number_of_connected_peers(), 0);
    }

    #[tokio::test]
    async fn reject_non_version_messages_before_handshake() {
        // start the node
        let mut node = test_node(vec![], Duration::new(10, 0), Duration::new(10, 0), Duration::new(10, 0)).await;
        node.start().await.unwrap();

        // start the fake node (peer) which is just a socket
        // note: the connection needs to be re-established as it is reset
        let mut peer_stream = TcpStream::connect(node.local_address().unwrap()).await.unwrap();

        // send a GetPeers message without a prior handshake established
        write_message_to_stream(Payload::GetPeers, &mut peer_stream).await;

        // verify the node rejected the message, the response to the peer is empty and the node's
        // state is unaltered
        assert_node_rejected_message(&node, &mut peer_stream).await;

        // GetMemoryPool
        let mut peer_stream = TcpStream::connect(node.local_address().unwrap()).await.unwrap();
        write_message_to_stream(Payload::GetMemoryPool, &mut peer_stream).await;
        assert_node_rejected_message(&node, &mut peer_stream).await;

        // GetBlock
        let mut peer_stream = TcpStream::connect(node.local_address().unwrap()).await.unwrap();
        let block_hash = BlockHeaderHash::new([0u8; 32].to_vec());
        write_message_to_stream(Payload::GetBlock(block_hash), &mut peer_stream).await;
        assert_node_rejected_message(&node, &mut peer_stream).await;

        // GetSync
        let mut peer_stream = TcpStream::connect(node.local_address().unwrap()).await.unwrap();
        let block_hash = BlockHeaderHash::new([0u8; 32].to_vec());
        write_message_to_stream(Payload::GetSync(vec![block_hash]), &mut peer_stream).await;
        assert_node_rejected_message(&node, &mut peer_stream).await;

        // Peers
        let mut peer_stream = TcpStream::connect(node.local_address().unwrap()).await.unwrap();
        let peers = vec![("127.0.0.1:0".parse().unwrap(), Utc::now())];
        write_message_to_stream(Payload::Peers(peers), &mut peer_stream).await;
        assert_node_rejected_message(&node, &mut peer_stream).await;

        // MemoryPool
        let mut peer_stream = TcpStream::connect(node.local_address().unwrap()).await.unwrap();
        let memory_pool = vec![vec![0u8, 10]];
        write_message_to_stream(Payload::MemoryPool(memory_pool), &mut peer_stream).await;
        assert_node_rejected_message(&node, &mut peer_stream).await;

        // Block
        let mut peer_stream = TcpStream::connect(node.local_address().unwrap()).await.unwrap();
        let block = vec![0u8, 10];
        write_message_to_stream(Payload::Block(block), &mut peer_stream).await;
        assert_node_rejected_message(&node, &mut peer_stream).await;

        // SyncBlock
        let mut peer_stream = TcpStream::connect(node.local_address().unwrap()).await.unwrap();
        let sync_block = vec![0u8, 10];
        write_message_to_stream(Payload::SyncBlock(sync_block), &mut peer_stream).await;
        assert_node_rejected_message(&node, &mut peer_stream).await;

        // Sync
        let mut peer_stream = TcpStream::connect(node.local_address().unwrap()).await.unwrap();
        let block_hash = BlockHeaderHash::new(vec![0u8; 32]);
        write_message_to_stream(Payload::Sync(vec![block_hash]), &mut peer_stream).await;
        assert_node_rejected_message(&node, &mut peer_stream).await;

        // Transaction
        let mut peer_stream = TcpStream::connect(node.local_address().unwrap()).await.unwrap();
        let transaction = vec![0u8, 10];
        write_message_to_stream(Payload::Transaction(transaction), &mut peer_stream).await;
        assert_node_rejected_message(&node, &mut peer_stream).await;

        // Verack
        let mut peer_stream = TcpStream::connect(node.local_address().unwrap()).await.unwrap();
        let verack = Verack::new(1u64);
        write_message_to_stream(Payload::Verack(verack), &mut peer_stream).await;
        assert_node_rejected_message(&node, &mut peer_stream).await;
    }

    async fn handshake(
        peer_sync_interval: Duration,
        block_sync_interval: Duration,
        transaction_sync_interval: Duration,
    ) -> (Server, TcpStream) {
        // start a test node and listen for incoming connections
        let mut node = test_node(
            vec![],
            peer_sync_interval,
            block_sync_interval,
            transaction_sync_interval,
        )
        .await;
        node.start().await.unwrap();
        let node_listener = node.local_address().unwrap();

        // set up a fake node (peer), which is just a socket
        let mut peer_stream = TcpStream::connect(&node_listener).await.unwrap();

        // register the addresses bound to the connection between the node and the peer
        let peer_address = peer_stream.local_addr().unwrap();

        // the peer initiates a handshake by sending a Version message
        let version = Payload::Version(Version::new(1u64, 1u32, 1u64, peer_address.port()));
        write_message_to_stream(version, &mut peer_stream).await;

        // at this point the node should have marked the peer as ' connecting'
        sleep(Duration::from_millis(200)).await;
        assert!(node.peers.is_connecting(&peer_address));

        // the buffer for peer's reads
        let mut peer_buf = [0u8; 64];

        // check if the peer has received the Verack message from the node
        let len = read_header(&mut peer_stream).await.unwrap().len();
        let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
        assert!(matches!(bincode::deserialize(&payload).unwrap(), Payload::Verack(_)));

        // check if it was followed by a Version message
        let len = read_header(&mut peer_stream).await.unwrap().len();
        let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
        let version = if let Payload::Version(version) = bincode::deserialize(&payload).unwrap() {
            version
        } else {
            unreachable!();
        };

        // in response to the Version, the peer sends a Verack message to finish the handshake
        let verack = Payload::Verack(Verack::new(version.nonce));
        write_message_to_stream(verack, &mut peer_stream).await;

        // the node should now have register the peer as 'connected'
        sleep(Duration::from_millis(200)).await;
        assert!(node.peers.is_connected(&peer_address));

        (node, peer_stream)
    }

    #[tokio::test]
    async fn sync_initiator_side() {
        // handshake between the fake node and full node
        let (node, mut peer_stream) =
            handshake(Duration::new(100, 0), Duration::new(10, 0), Duration::new(100, 0)).await;

        // the buffer for peer's reads
        let mut peer_buf = [0u8; 64];

        // check GetSync message was received
        let len = read_header(&mut peer_stream).await.unwrap().len();
        let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
        assert!(matches!(bincode::deserialize(&payload).unwrap(), Payload::GetSync(..)));

        let block_1_header_hash = BlockHeaderHash::new(BLOCK_1_HEADER_HASH.to_vec());
        let block_2_header_hash = BlockHeaderHash::new(BLOCK_2_HEADER_HASH.to_vec());

        let block_header_hashes = vec![block_1_header_hash.clone(), block_2_header_hash.clone()];

        // respond to GetSync with Sync message containing the block header hashes of the missing
        // blocks
        let sync = Payload::Sync(block_header_hashes);
        write_message_to_stream(sync, &mut peer_stream).await;

        // make sure both GetBlock messages are received
        let len = read_header(&mut peer_stream).await.unwrap().len();
        let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
        let block_hash = if let Payload::GetBlock(block_hash) = bincode::deserialize(&payload).unwrap() {
            block_hash
        } else {
            unreachable!();
        };

        assert_eq!(block_hash, block_1_header_hash);

        let len = read_header(&mut peer_stream).await.unwrap().len();
        let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
        let block_hash = if let Payload::GetBlock(block_hash) = bincode::deserialize(&payload).unwrap() {
            block_hash
        } else {
            unreachable!();
        };

        assert_eq!(block_hash, block_2_header_hash);

        // respond with the full blocks
        let block_1 = Payload::Block(BLOCK_1.to_vec());
        write_message_to_stream(block_1, &mut peer_stream).await;

        let block_2 = Payload::Block(BLOCK_2.to_vec());
        write_message_to_stream(block_2, &mut peer_stream).await;

        sleep(Duration::from_millis(200)).await;

        // check the blocks have been added to the node's chain
        assert!(
            node.environment
                .storage()
                .read()
                .block_hash_exists(&block_1_header_hash)
        );

        assert!(
            node.environment
                .storage()
                .read()
                .block_hash_exists(&block_2_header_hash)
        );
    }

    #[tokio::test]
    async fn sync_responder_side() {
        // handshake between the fake and full node
        let (node, mut peer_stream) =
            handshake(Duration::new(100, 0), Duration::new(10, 0), Duration::new(100, 0)).await;

        // insert block into node
        let block_struct_1 = snarkvm_objects::Block::deserialize(&BLOCK_1).unwrap();
        node.environment
            .consensus_parameters()
            .receive_block(
                node.environment.dpc_parameters(),
                &node.environment.storage().read(),
                &mut node.environment.memory_pool().lock(),
                &block_struct_1,
            )
            .unwrap();

        // send a GetSync with an empty vec as only the genesis block is in the ledger
        let get_sync = Payload::GetSync(vec![]);
        write_message_to_stream(get_sync, &mut peer_stream).await;

        // the buffer for peer's reads
        let mut peer_buf = [0u8; 4096];

        // receive a Sync message from the node with the block header
        let len = read_header(&mut peer_stream).await.unwrap().len();
        let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
        let sync = if let Payload::Sync(sync) = bincode::deserialize(&payload).unwrap() {
            sync
        } else {
            unreachable!();
        };

        let block_header_hash = sync.first().unwrap();

        // check it matches the block inserted into the node's ledger
        assert_eq!(*block_header_hash, block_struct_1.header.get_hash());

        // request the block from the node
        let get_block = Payload::GetBlock(block_header_hash.clone());
        write_message_to_stream(get_block, &mut peer_stream).await;

        // receive a SyncBlock message with the requested block
        let len = read_header(&mut peer_stream).await.unwrap().len();
        let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
        let block = if let Payload::SyncBlock(block) = bincode::deserialize(&payload).unwrap() {
            block
        } else {
            unreachable!();
        };
        let block = snarkvm_objects::Block::deserialize(&block).unwrap();

        assert_eq!(block, block_struct_1);
    }
}
