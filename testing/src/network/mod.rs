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
pub mod encryption;

#[cfg(test)]
pub mod sync;

use crate::{
    consensus::{FIXTURE_VK, TEST_CONSENSUS},
    dpc::load_verifying_parameters,
};

use snarkos_network::{
    connection_reader::ConnReader,
    connection_writer::ConnWriter,
    errors::{message::*, network::*},
    external::message::*,
    Consensus,
    Environment,
    Server,
    MAX_MESSAGE_SIZE,
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
            TEST_CONSENSUS.clone(),
            load_verifying_parameters(),
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
    let environment = test_environment(setup);
    let mut node = Server::new(environment).await.unwrap();
    node.start().await.unwrap();

    node
}

pub struct FakeNode {
    reader: ConnReader,
    writer: ConnWriter,
}

impl FakeNode {
    pub fn new(stream: TcpStream, peer_addr: SocketAddr, noise: snow::TransportState) -> Self {
        let buffer = vec![0u8; snarkos_network::MAX_MESSAGE_SIZE].into_boxed_slice();
        let noise = Arc::new(Mutex::new(noise));
        let (reader, writer) = stream.into_split();

        let reader = ConnReader::new(peer_addr, reader, buffer.clone(), noise.clone());

        let writer = ConnWriter::new(peer_addr, writer, buffer, noise);

        Self { reader, writer }
    }

    pub async fn read_payload(&mut self) -> Result<Payload, NetworkError> {
        let message = self.reader.read_message().await?;

        Ok(message.payload)
    }

    pub async fn write_message(&mut self, payload: &Payload) {
        self.writer.write_message(payload).await.unwrap();
    }
}

pub async fn handshaken_node_and_peer(node_setup: TestSetup) -> (Server, FakeNode) {
    // start a test node and listen for incoming connections
    let node = test_node(node_setup).await;
    let node_listener = node.local_address().unwrap();

    // set up a fake node (peer), which is basically just a socket
    let mut peer_stream = TcpStream::connect(&node_listener).await.unwrap();

    // register the addresses bound to the connection between the node and the peer
    let peer_addr = peer_stream.local_addr().unwrap();

    let builder = snow::Builder::with_resolver(
        snarkos_network::HANDSHAKE_PATTERN.parse().unwrap(),
        Box::new(snow::resolvers::SodiumResolver),
    );
    let static_key = builder.generate_keypair().unwrap().private;
    let noise_builder = builder
        .local_private_key(&static_key)
        .psk(3, snarkos_network::HANDSHAKE_PSK);
    let mut noise = noise_builder.build_initiator().unwrap();
    let mut buffer: Box<[u8]> = vec![0u8; snarkos_network::NOISE_BUF_LEN].into();
    let mut buf = [0u8; snarkos_network::NOISE_BUF_LEN]; // a temporary intermediate buffer to decrypt from

    // -> e
    let len = noise.write_message(&[], &mut buffer).unwrap();
    peer_stream.write_all(&[len as u8]).await.unwrap();
    peer_stream.write_all(&buffer[..len]).await.unwrap();

    // <- e, ee, s, es
    peer_stream.read_exact(&mut buf[..1]).await.unwrap();
    let len = buf[0] as usize;
    let len = peer_stream.read_exact(&mut buf[..len]).await.unwrap();
    let len = noise.read_message(&buf[..len], &mut buffer).unwrap();
    let _node_version: Version = bincode::deserialize(&buffer[..len]).unwrap();

    // -> s, se, psk
    let peer_version = bincode::serialize(&Version::new(1u64, peer_addr.port())).unwrap(); // TODO (raychu86): Establish a formal node version.
    let len = noise.write_message(&peer_version, &mut buffer).unwrap();
    peer_stream.write_all(&[len as u8]).await.unwrap();
    peer_stream.write_all(&buffer[..len]).await.unwrap();

    let noise = noise.into_transport_mode().unwrap();

    let fake_node = FakeNode::new(peer_stream, peer_addr, noise);

    (node, fake_node)
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
