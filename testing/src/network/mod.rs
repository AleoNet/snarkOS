// Copyright (C) 2019-2021 Aleo Systems Inc.
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

pub mod blocks;
pub use blocks::*;

#[cfg(test)]
pub mod sync;

use crate::consensus::{FIXTURE, FIXTURE_VK, TEST_CONSENSUS};

use snarkos_network::{
    errors::message::*,
    external::{message::*, MAX_MESSAGE_SIZE},
    Consensus,
    Environment,
    Server,
};

use parking_lot::{Mutex, RwLock};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

/// Returns a random tcp socket address and binds it to a listener
pub async fn random_bound_address() -> (SocketAddr, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    (addr, listener)
}

#[macro_export]
macro_rules! wait_until {
    ($limit_secs: expr, $condition: expr) => {
        let now = std::time::Instant::now();
        loop {
            if $condition {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if now.elapsed() > std::time::Duration::from_secs($limit_secs) {
                panic!("timed out!");
            }
        }
    };
}

#[derive(Clone)]
pub struct ConsensusSetup {
    pub is_miner: bool,
    pub block_sync_interval: u64,
    pub tx_sync_interval: u64,
}

impl ConsensusSetup {
    pub fn new(is_miner: bool, block_sync_interval: u64, tx_sync_interval: u64) -> Self {
        Self {
            is_miner,
            block_sync_interval,
            tx_sync_interval,
        }
    }
}

impl Default for ConsensusSetup {
    fn default() -> Self {
        Self {
            is_miner: false,
            block_sync_interval: 600,
            tx_sync_interval: 600,
        }
    }
}

#[derive(Clone)]
pub struct TestSetup {
    pub socket_address: Option<SocketAddr>,
    pub consensus_setup: Option<ConsensusSetup>,
    pub peer_sync_interval: u64,
    pub min_peers: u16,
    pub max_peers: u16,
    pub is_bootnode: bool,
    pub bootnodes: Vec<String>,
}

impl TestSetup {
    pub fn new(
        socket_address: Option<SocketAddr>,
        consensus_setup: Option<ConsensusSetup>,
        peer_sync_interval: u64,
        min_peers: u16,
        max_peers: u16,
        is_bootnode: bool,
        bootnodes: Vec<String>,
    ) -> Self {
        Self {
            socket_address,
            consensus_setup,
            peer_sync_interval,
            min_peers,
            max_peers,
            is_bootnode,
            bootnodes,
        }
    }
}

impl Default for TestSetup {
    fn default() -> Self {
        Self {
            socket_address: None,
            consensus_setup: Some(Default::default()),
            peer_sync_interval: 600,
            min_peers: 1,
            max_peers: 100,
            is_bootnode: false,
            bootnodes: vec![],
        }
    }
}

/// Returns an `Environment` struct with given arguments
pub fn test_environment(setup: TestSetup) -> Environment {
    let consensus = if let Some(ref setup) = setup.consensus_setup {
        Some(Consensus::new(
            Arc::new(RwLock::new(FIXTURE_VK.ledger())),
            Arc::new(Mutex::new(snarkos_consensus::MemoryPool::new())),
            Arc::new(TEST_CONSENSUS.clone()),
            Arc::new(FIXTURE.parameters.clone()),
            setup.is_miner,
            Duration::from_secs(setup.block_sync_interval),
            Duration::from_secs(setup.tx_sync_interval),
        ))
    } else {
        None
    };

    Environment::new(
        consensus,
        setup.socket_address,
        setup.min_peers,
        setup.max_peers,
        setup.bootnodes,
        setup.is_bootnode,
        Duration::from_secs(setup.peer_sync_interval),
    )
    .unwrap()
}

/// Starts a node with the specified bootnodes.
pub async fn test_node(setup: TestSetup) -> Server {
    let is_miner = setup.consensus_setup.as_ref().map(|c| c.is_miner) == Some(true);
    let environment = test_environment(setup);
    let mut node = Server::new(environment).await.unwrap();
    node.start().await.unwrap();

    if is_miner {
        // TODO(ljedrz/nkls): spawn a miner
    }

    node
}

pub async fn handshaken_node_and_peer(node_setup: TestSetup) -> (Server, TcpStream) {
    // start a test node and listen for incoming connections
    let node = test_node(node_setup).await;
    let node_listener = node.local_address().unwrap();

    // set up a fake node (peer), which is just a socket
    let mut peer_stream = TcpStream::connect(&node_listener).await.unwrap();

    // register the addresses bound to the connection between the node and the peer
    let peer_address = peer_stream.local_addr().unwrap();

    // the peer initiates a handshake by sending a Version message
    let version = Payload::Version(Version::new(1u64, 1u64, peer_address.port()));
    write_message_to_stream(version, &mut peer_stream).await;

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
    let verack = Payload::Verack(version.nonce);
    write_message_to_stream(verack, &mut peer_stream).await;

    (node, peer_stream)
}

pub async fn read_payload<'a, T: AsyncRead + Unpin>(
    stream: &mut T,
    buffer: &'a mut [u8],
) -> Result<&'a [u8], MessageError> {
    stream.read_exact(buffer).await?;

    Ok(buffer)
}

pub async fn read_header<T: AsyncRead + Unpin>(stream: &mut T) -> Result<MessageHeader, MessageHeaderError> {
    let mut header_arr = [0u8; 4];
    stream.read_exact(&mut header_arr).await?;
    let header = MessageHeader::from(header_arr);

    if header.len as usize > MAX_MESSAGE_SIZE {
        Err(MessageHeaderError::TooBig(header.len as usize, MAX_MESSAGE_SIZE))
    } else {
        Ok(header)
    }
}

pub async fn write_message_to_stream(payload: Payload, peer_stream: &mut TcpStream) {
    let payload = bincode::serialize(&payload).unwrap();
    let header = MessageHeader {
        len: payload.len() as u32,
    }
    .as_bytes();
    peer_stream.write_all(&header[..]).await.unwrap();
    peer_stream.write_all(&payload).await.unwrap();
    peer_stream.flush().await.unwrap();
}
