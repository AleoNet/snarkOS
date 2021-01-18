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

mod common;
use common::{test_node, write_message_to_stream};

use snarkos_network::{
    external::{message::*, Verack, Version},
    Server,
};
use snarkos_testing::network::{read_header, read_payload};

use snarkvm_objects::block_header_hash::BlockHeaderHash;

use std::time::Duration;

use chrono::Utc;
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    time::sleep,
};

#[tokio::test]
async fn handshake_responder_side() {
    // start a test node and listen for incoming connections
    let mut node = test_node(
        vec![],
        Duration::from_secs(10),
        Duration::from_secs(10),
        Duration::from_secs(10),
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
    // note: using the smallest allowed interval for peer sync
    let mut node = test_node(
        vec![peer_address.to_string()],
        Duration::from_secs(2),
        Duration::from_secs(10),
        Duration::from_secs(10),
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
    let mut node = test_node(
        vec![],
        Duration::from_secs(10),
        Duration::from_secs(10),
        Duration::from_secs(10),
    )
    .await;
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
