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

pub mod blocks;
pub use blocks::*;

#[cfg(test)]
pub mod sync;

use crate::{
    consensus::{FIXTURE_VK, TEST_CONSENSUS},
    dpc::load_verifying_parameters,
};

use snarkos_consensus::MerkleTreeLedger;
use snarkos_network::{
    errors::message::*,
    external::{message::*, Verack, Version, MAX_MESSAGE_SIZE},
    Environment,
    Server,
};
use snarkvm_dpc::{instantiated::Components, PublicParameters};

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

/// Returns an `Environment` struct with given arguments
pub fn test_environment(
    socket_address: Option<SocketAddr>,
    bootnodes: Vec<String>,
    storage: Arc<RwLock<MerkleTreeLedger>>,
    parameters: PublicParameters<Components>,
    peer_sync_interval: Duration,
    block_sync_interval: Duration,
    transaction_sync_interval: Duration,
) -> Environment {
    let memory_pool = snarkos_consensus::MemoryPool::new();
    let memory_pool_lock = Arc::new(Mutex::new(memory_pool));
    let consensus = TEST_CONSENSUS.clone();
    let min_peers = 1;
    let max_peers = 10;
    let is_bootnode = false;
    let is_miner = false;

    Environment::new(
        storage,
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
    .unwrap()
}

/// Starts a node with the specified bootnodes.
pub async fn test_node(
    bootnodes: Vec<String>,
    peer_sync_interval: Duration,
    block_sync_interval: Duration,
    transaction_sync_interval: Duration,
) -> Server {
    let storage = Arc::new(RwLock::new(FIXTURE_VK.ledger()));
    let environment = test_environment(
        None,
        bootnodes,
        storage,
        load_verifying_parameters(),
        peer_sync_interval,
        block_sync_interval,
        transaction_sync_interval,
    );

    let mut node = Server::new(environment).await.unwrap();
    node.start().await.unwrap();

    node
}

pub async fn handshake(
    peer_sync_interval: Duration,
    block_sync_interval: Duration,
    transaction_sync_interval: Duration,
) -> (Server, TcpStream) {
    // start a test node and listen for incoming connections
    let node = test_node(
        vec![],
        peer_sync_interval,
        block_sync_interval,
        transaction_sync_interval,
    )
    .await;
    let node_listener = node.local_address().unwrap();

    // set up a fake node (peer), which is just a socket
    let mut peer_stream = TcpStream::connect(&node_listener).await.unwrap();

    // register the addresses bound to the connection between the node and the peer
    let peer_address = peer_stream.local_addr().unwrap();

    // the peer initiates a handshake by sending a Version message
    let version = Payload::Version(Version::new(1u64, 1u32, 1u64, peer_address.port()));
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
    let verack = Payload::Verack(Verack::new(version.nonce));
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
