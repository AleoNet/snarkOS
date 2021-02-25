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

use rand::{distributions::Standard, thread_rng, Rng};
use snarkos_testing::{
    network::{handshaken_node_and_peer, spawn_2_fake_nodes, test_node, TestSetup},
    wait_until,
};
use tokio::{io::AsyncWriteExt, net::TcpStream};

#[tokio::test]
async fn fuzzing_zeroes_pre_handshake() {
    let node_setup = TestSetup {
        consensus_setup: None,
        is_bootnode: true, // same rules for establishing connections and reading messages as a regular node, but lighter
        ..Default::default()
    };
    let node = test_node(node_setup).await;
    let node_addr = node.environment.local_address().unwrap();

    let mut stream = TcpStream::connect(node_addr).await.unwrap();
    wait_until!(1, node.peer_book.read().number_of_connecting_peers() == 1);

    let _ = stream.write_all(&vec![0u8; 64]).await;
    wait_until!(1, node.peer_book.read().number_of_connecting_peers() == 0);
}

#[tokio::test]
async fn fuzzing_zeroes_post_handshake() {
    let node_setup = TestSetup {
        consensus_setup: None,
        is_bootnode: true,
        ..Default::default()
    };
    let (node, fake_node) = handshaken_node_and_peer(node_setup).await;
    wait_until!(1, node.peer_book.read().number_of_connected_peers() == 1);

    fake_node.write_bytes(&vec![0u8; 64]).await;
    wait_until!(1, node.peer_book.read().number_of_connected_peers() == 0);
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn fuzzing_valid_header_pre_handshake() {
    // tracing_subscriber::fmt::init();

    let node_setup = TestSetup {
        consensus_setup: None,
        is_bootnode: true,
        ..Default::default()
    };
    let node = test_node(node_setup).await;
    let node_addr = node.environment.local_address().unwrap();

    loop {
        let random_len: usize = thread_rng().gen_range(1, 64 * 1024);
        let random_payload: Vec<u8> = (&mut thread_rng()).sample_iter(Standard).take(random_len).collect();

        let mut stream = TcpStream::connect(node_addr).await.unwrap();
        let _ = stream.write_u32(random_len as u32).await;
        let _ = stream.write_all(&random_payload).await;
    }
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn fuzzing_valid_header_post_handshake() {
    // tracing_subscriber::fmt::init();

    let (node1, mut node2) = spawn_2_fake_nodes().await;

    tokio::spawn(async move {
        loop {
            let _ = node2.read_payload().await;
        }
    });

    loop {
        let random_len: usize = thread_rng().gen_range(1, 64 * 1024);
        let random_payload: Vec<u8> = (&mut thread_rng()).sample_iter(Standard).take(random_len).collect();

        node1.write_bytes(&(random_len as u32).to_be_bytes()).await;
        node1.write_bytes(&random_payload).await;
    }
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn fuzzing_pre_handshake() {
    // tracing_subscriber::fmt::init();

    let node_setup = TestSetup {
        consensus_setup: None,
        is_bootnode: true,
        ..Default::default()
    };
    let node = test_node(node_setup).await;
    let node_addr = node.environment.local_address().unwrap();

    loop {
        let random_len: usize = thread_rng().gen_range(1, 64 * 1024);
        let random_bytes: Vec<u8> = (&mut thread_rng()).sample_iter(Standard).take(random_len).collect();

        let mut stream = TcpStream::connect(node_addr).await.unwrap();
        let _ = stream.write_all(&random_bytes).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn fuzzing_post_handshake() {
    tracing_subscriber::fmt::init();

    let (node1, mut node2) = spawn_2_fake_nodes().await;

    tokio::spawn(async move {
        loop {
            let _ = node2.read_payload().await;
        }
    });

    for _ in 0..10000 {
        let random_len: usize = thread_rng().gen_range(1, 64 * 1024);
        let random_bytes: Vec<u8> = (&mut thread_rng()).sample_iter(Standard).take(random_len).collect();

        node1.write_bytes(&random_bytes).await;
    }
}
