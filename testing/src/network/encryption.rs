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

use crate::network::FakeNode;

use snarkos_network::external::message::{Payload, Version};

use rand::{distributions::Standard, thread_rng, Rng};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use std::net::SocketAddr;

async fn spawn_2_fake_nodes() -> (FakeNode, FakeNode) {
    // set up listeners and establish addresses
    let node0_listener = TcpListener::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap())
        .await
        .unwrap();
    let node0_addr = node0_listener.local_addr().unwrap();
    let node0_listening_task = tokio::spawn(async move { node0_listener.accept().await.unwrap() });

    // set up streams
    let mut node1_stream = TcpStream::connect(&node0_addr).await.unwrap();
    let (mut node0_stream, node1_addr) = node0_listening_task.await.unwrap();

    // node0's noise - initiator
    let builder = snow::Builder::with_resolver(
        snarkos_network::HANDSHAKE_PATTERN.parse().unwrap(),
        Box::new(snow::resolvers::SodiumResolver),
    );
    let static_key = builder.generate_keypair().unwrap().private;
    let noise_builder = builder
        .local_private_key(&static_key)
        .psk(3, snarkos_network::HANDSHAKE_PSK);
    let mut node0_noise = noise_builder.build_initiator().unwrap();

    // node1's noise - responder
    let builder = snow::Builder::with_resolver(
        snarkos_network::HANDSHAKE_PATTERN.parse().unwrap(),
        Box::new(snow::resolvers::SodiumResolver),
    );
    let static_key = builder.generate_keypair().unwrap().private;
    let noise_builder = builder
        .local_private_key(&static_key)
        .psk(3, snarkos_network::HANDSHAKE_PSK);
    let mut node1_noise = noise_builder.build_responder().unwrap();

    // shared bits
    let mut buffer: Box<[u8]> = vec![0u8; snarkos_network::NOISE_BUF_LEN].into();
    let mut buf = [0u8; snarkos_network::NOISE_BUF_LEN];

    // -> e (node0)
    let len = node0_noise.write_message(&[], &mut buffer).unwrap();
    node0_stream.write_all(&[len as u8]).await.unwrap();
    node0_stream.write_all(&buffer[..len]).await.unwrap();

    // <- e (node1)
    node1_stream.read_exact(&mut buf[..1]).await.unwrap();
    let len = buf[0] as usize;
    let len = node1_stream.read_exact(&mut buf[..len]).await.unwrap();
    node1_noise.read_message(&buf[..len], &mut buffer).unwrap();

    // -> e, ee, s, es (node1)
    let version = bincode::serialize(&Version::new(1u64, node1_addr.port())).unwrap();
    let len = node1_noise.write_message(&version, &mut buffer).unwrap();
    node1_stream.write_all(&[len as u8]).await.unwrap();
    node1_stream.write_all(&buffer[..len]).await.unwrap();

    // <- e, ee, s, es (node0)
    node0_stream.read_exact(&mut buf[..1]).await.unwrap();
    let len = buf[0] as usize;
    let len = node0_stream.read_exact(&mut buf[..len]).await.unwrap();
    let len = node0_noise.read_message(&buf[..len], &mut buffer).unwrap();
    let _version: Version = bincode::deserialize(&buffer[..len]).unwrap();

    // -> s, se, psk (node0)
    let peer_version = bincode::serialize(&Version::new(1u64, node0_addr.port())).unwrap();
    let len = node0_noise.write_message(&peer_version, &mut buffer).unwrap();
    node0_stream.write_all(&[len as u8]).await.unwrap();
    node0_stream.write_all(&buffer[..len]).await.unwrap();

    // <- e, ee, s, es (node1)
    node1_stream.read_exact(&mut buf[..1]).await.unwrap();
    let len = buf[0] as usize;
    let len = node1_stream.read_exact(&mut buf[..len]).await.unwrap();
    let len = node1_noise.read_message(&buf[..len], &mut buffer).unwrap();
    let _version: Version = bincode::deserialize(&buffer[..len]).unwrap();

    let node0_noise = node0_noise.into_transport_mode().unwrap();
    let node1_noise = node1_noise.into_transport_mode().unwrap();

    let node0 = FakeNode::new(node0_stream, node0_addr, node0_noise);
    let node1 = FakeNode::new(node1_stream, node1_addr, node1_noise);

    (node0, node1)
}

// note: this test is "byte-tight"; if there's any changes to block serialization or MAX_MESSAGE_SIZE is
// increased without increasing the size of the subtracted overhead, the test will fail
#[tokio::test]
async fn encrypt_and_decrypt_a_big_payload() {
    let (mut node0, mut node1) = spawn_2_fake_nodes().await;

    // account for the overhead of serialization and noise tags
    let block_size = snarkos_network::MAX_MESSAGE_SIZE - 2076;

    // create a big block containing random data
    let fake_block_bytes: Vec<u8> = (&mut thread_rng()).sample_iter(Standard).take(block_size).collect();
    let big_block = Payload::Block(fake_block_bytes.clone());

    let reading_task = tokio::spawn(async move { node1.read_payload().await.unwrap() });

    // send it from node0 to node1
    node0.write_message(&big_block).await;
    let payload = reading_task.await.unwrap();

    // check if node1 received the expected data
    if let Payload::Block(bytes) = payload {
        assert!(bytes == fake_block_bytes);
    } else {
        panic!("wrong payload received");
    }
}

#[tokio::test]
async fn encrypt_and_decrypt_small_payloads() {
    let (mut node0, mut node1) = spawn_2_fake_nodes().await;

    let mut rng = thread_rng();

    for _ in 0..100 {
        // create a small block containing random data
        let block_size: u8 = rng.gen();
        let fake_block_bytes: Vec<u8> = (&mut rng).sample_iter(Standard).take(block_size as usize).collect();
        let big_block = Payload::Block(fake_block_bytes.clone());

        // send it from node0 to node1
        node0.write_message(&big_block).await;
        let payload = node1.read_payload().await.unwrap();

        // check if node1 received the expected data
        if let Payload::Block(bytes) = payload {
            assert!(bytes == fake_block_bytes);
        } else {
            panic!("wrong payload received");
        }
    }
}
