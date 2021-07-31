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

use snarkos_network::{MessageHeader, Payload, Version};
use snarkvm::ledger::BlockHeaderHash;

use rand::{distributions::Standard, thread_rng, Rng};
use snarkos_testing::{
    network::{handshaken_node_and_peer, spawn_2_fake_nodes, test_node, TestSetup},
    wait_until,
};
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex};

use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub const ITERATIONS: usize = 1000;
pub const CORRUPTION_PROBABILITY: f64 = 0.1;

fn corrupt_bytes(serialized: &[u8]) -> Vec<u8> {
    let mut rng = thread_rng();

    serialized
        .iter()
        .map(|byte| {
            if rng.gen_bool(CORRUPTION_PROBABILITY) {
                rng.gen()
            } else {
                *byte
            }
        })
        .collect()
}

#[tokio::test]
async fn fuzzing_zeroes_pre_handshake() {
    let node_setup = TestSetup {
        consensus_setup: None,
        is_bootnode: true, // same rules for establishing connections and reading messages as a regular node, but lighter
        ..Default::default()
    };
    let node = test_node(node_setup).await;
    let node_addr = node.local_address().unwrap();

    let mut stream = TcpStream::connect(node_addr).await.unwrap();
    wait_until!(1, node.peer_book.get_active_peer_count() == 1);

    let _ = stream.write_all(&[0u8; 64]).await;
    wait_until!(1, node.peer_book.get_active_peer_count() == 0);
}

#[tokio::test]
async fn fuzzing_zeroes_post_handshake() {
    let node_setup = TestSetup {
        consensus_setup: None,
        is_bootnode: true,
        ..Default::default()
    };
    let (node, mut fake_node) = handshaken_node_and_peer(node_setup).await;
    wait_until!(1, node.peer_book.get_active_peer_count() == 1);

    fake_node.write_bytes(&[0u8; 64]).await;
    wait_until!(1, node.peer_book.get_active_peer_count() == 0);
}

#[tokio::test]
async fn fuzzing_valid_header_pre_handshake() {
    // tracing_subscriber::fmt::init();

    let node_setup = TestSetup {
        consensus_setup: None,
        is_bootnode: true,
        ..Default::default()
    };
    let node = test_node(node_setup).await;
    let node_addr = node.local_address().unwrap();

    for _ in 0..ITERATIONS {
        let random_len: usize = thread_rng().gen_range(1..(64 * 1024));
        let random_payload: Vec<u8> = (&mut thread_rng()).sample_iter(Standard).take(random_len).collect();

        let mut stream = TcpStream::connect(node_addr).await.unwrap();
        let _ = stream.write_u32(random_len as u32).await;
        let _ = stream.write_all(&random_payload).await;
    }
}

#[tokio::test]
async fn fuzzing_valid_header_post_handshake() {
    // tracing_subscriber::fmt::init();

    let (mut node1, mut node2) = spawn_2_fake_nodes().await;
    let write_finished = Arc::new(AtomicBool::new(false));
    let should_exit = Arc::clone(&write_finished);

    let handle = tokio::spawn(async move {
        loop {
            if should_exit.load(Ordering::Relaxed) {
                break;
            }

            let _ = node2.read_payload().await;
        }
    });

    for _ in 0..ITERATIONS {
        let random_len: usize = thread_rng().gen_range(1..(64 * 1024));
        let random_payload: Vec<u8> = (&mut thread_rng()).sample_iter(Standard).take(random_len).collect();

        node1.write_bytes(&(random_len as u32).to_be_bytes()).await;
        node1.write_bytes(&random_payload).await;
    }

    write_finished.store(true, Ordering::Relaxed);
    handle.abort();
    handle.await.ok();
}

#[tokio::test]
async fn fuzzing_pre_handshake() {
    // tracing_subscriber::fmt::init();

    let node_setup = TestSetup {
        consensus_setup: None,
        is_bootnode: true,
        ..Default::default()
    };
    let node = test_node(node_setup).await;
    let node_addr = node.local_address().unwrap();

    for _ in 0..ITERATIONS {
        let random_len: usize = thread_rng().gen_range(1..(64 * 1024));
        let random_bytes: Vec<u8> = (&mut thread_rng()).sample_iter(Standard).take(random_len).collect();

        let mut stream = TcpStream::connect(node_addr).await.unwrap();
        let _ = stream.write_all(&random_bytes).await;
    }
}

#[tokio::test]
async fn fuzzing_post_handshake() {
    // tracing_subscriber::fmt::init();

    let (mut node1, mut node2) = spawn_2_fake_nodes().await;
    let write_finished = Arc::new(AtomicBool::new(false));
    let should_exit = Arc::clone(&write_finished);

    let handle = tokio::spawn(async move {
        loop {
            if should_exit.load(Ordering::Relaxed) {
                break;
            }

            let _ = node2.read_payload().await;
        }
    });

    for _ in 0..ITERATIONS {
        let random_len: usize = thread_rng().gen_range(1..(64 * 1024));
        let random_bytes: Vec<u8> = (&mut thread_rng()).sample_iter(Standard).take(random_len).collect();

        node1.write_bytes(&random_bytes).await;
    }

    write_finished.store(true, Ordering::Relaxed);
    handle.abort();
    handle.await.ok();
}

#[tokio::test]
async fn fuzzing_corrupted_version_pre_handshake() {
    // tracing_subscriber::fmt::init();

    let node_setup = TestSetup {
        consensus_setup: None,
        ..Default::default()
    };

    let node = test_node(node_setup).await;
    let node_addr = node.local_address().unwrap();

    for i in 0..ITERATIONS {
        let mut stream = TcpStream::connect(node_addr).await.unwrap();
        let version = Version::serialize(&Version::new(
            snarkos_network::PROTOCOL_VERSION,
            stream.local_addr().unwrap().port(),
            i as u64,
        ))
        .unwrap();

        let corrupted_version = corrupt_bytes(&version);

        let header = MessageHeader::from(corrupted_version.len());

        let _ = stream.write_all(&header.as_bytes()).await;
        let _ = stream.write_all(&corrupted_version).await;
    }

    wait_until!(3, node.peer_book.get_active_peer_count() == 0);
}

#[tokio::test]
async fn fuzzing_corrupted_version_post_handshake() {
    // tracing_subscriber::fmt::init();

    let (mut node1, mut node2) = spawn_2_fake_nodes().await;
    let write_finished = Arc::new(AtomicBool::new(false));
    let should_exit = Arc::clone(&write_finished);

    let handle = tokio::spawn(async move {
        loop {
            if should_exit.load(Ordering::Relaxed) {
                break;
            }

            let _ = node2.read_payload().await;
        }
    });

    let version = Version::serialize(&Version::new(snarkos_network::PROTOCOL_VERSION, 4141, 0)).unwrap();
    for _ in 0..ITERATIONS {
        // Replace a random percentage of random bytes at random indices in the serialised message.
        let corrupted_version = corrupt_bytes(&version);

        let header = MessageHeader::from(corrupted_version.len());

        node1.write_bytes(&header.as_bytes()).await;
        node1.write_bytes(&corrupted_version).await;
    }

    write_finished.store(true, Ordering::Relaxed);
    handle.abort();
    handle.await.ok();
}

#[tokio::test]
async fn fuzzing_corrupted_empty_payloads_pre_handshake() {
    // All messages should get rejected pre-handshake, however, here we fuzz to search for
    // potential breakage during deserialisation.

    // tracing_subscriber::fmt::init();

    let node_setup = TestSetup {
        consensus_setup: None,
        ..Default::default()
    };

    let node = test_node(node_setup).await;
    let node_addr = node.local_address().unwrap();

    for payload in &[Payload::GetMemoryPool, Payload::GetPeers, Payload::Pong] {
        let serialized = Payload::serialize(payload).unwrap();

        for _ in 0..ITERATIONS {
            let corrupted_payload = corrupt_bytes(&serialized);

            let header = MessageHeader::from(corrupted_payload.len());

            let mut stream = TcpStream::connect(node_addr).await.unwrap();
            let _ = stream.write_all(&header.as_bytes()).await;
            let _ = stream.write_all(&corrupted_payload).await;
        }
    }

    wait_until!(3, node.peer_book.get_active_peer_count() == 0);
}

#[tokio::test]
async fn fuzzing_corrupted_empty_payloads_post_handshake() {
    // tracing_subscriber::fmt::init();

    let (mut node1, mut node2) = spawn_2_fake_nodes().await;
    let write_finished = Arc::new(AtomicBool::new(false));
    let should_exit = Arc::clone(&write_finished);

    let handle = tokio::spawn(async move {
        loop {
            if should_exit.load(Ordering::Relaxed) {
                break;
            }

            let _ = node2.read_payload().await;
        }
    });

    for payload in &[Payload::GetMemoryPool, Payload::GetPeers, Payload::Pong] {
        let serialized = Payload::serialize(payload).unwrap();

        for _ in 0..ITERATIONS {
            let corrupted_payload = corrupt_bytes(&serialized);

            let header = MessageHeader::from(corrupted_payload.len());

            node1.write_bytes(&header.as_bytes()).await;
            node1.write_bytes(&corrupted_payload).await;
        }
    }

    write_finished.store(true, Ordering::Relaxed);
    handle.await.unwrap();
}

// Using a multi-threaded rt for this test notably improves performance.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn fuzzing_corrupted_payloads_with_bodies_pre_handshake() {
    // tracing_subscriber::fmt::init();

    let node_setup = TestSetup {
        consensus_setup: None,
        ..Default::default()
    };

    let node = test_node(node_setup).await;
    let node_addr = node.local_address().unwrap();

    let mut rng = thread_rng();
    let random_len: usize = rng.gen_range(1..(64 * 1024));
    let blob: Vec<u8> = rng.sample_iter(Standard).take(random_len).collect();

    let addrs: Vec<SocketAddr> = [
        "0.0.0.0:0",
        "127.0.0.1:4141",
        "192.168.1.1:14131",
        "[::1]:0",
        "[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:14131",
        "[::ffff:192.0.2.128]:4141",
    ]
    .iter()
    .map(|addr| addr.parse().unwrap())
    .collect();

    for payload in &[
        Payload::Block(blob.clone()),
        Payload::MemoryPool(vec![blob.clone(); 10]),
        Payload::SyncBlock(blob.clone()),
        Payload::Transaction(blob.clone()),
        Payload::Peers(addrs.clone()),
        Payload::Ping(thread_rng().gen()),
    ] {
        let serialized = Payload::serialize(payload).unwrap();

        let mut future_set = vec![];
        for _ in 0..100 {
            let serialized = serialized.clone();
            future_set.push(tokio::spawn(async move {
                let corrupted_payload = corrupt_bytes(&serialized);

                let header = MessageHeader::from(corrupted_payload.len());

                let mut stream = TcpStream::connect(node_addr).await.unwrap();
                let _ = stream.write_all(&header.as_bytes()).await;
                let _ = stream.write_all(&corrupted_payload).await;
            }));
        }
        futures::future::join_all(future_set).await;
    }

    wait_until!(3, node.peer_book.get_active_peer_count() == 0);
}

// Using a multi-threaded rt for this test notably improves performance.
#[tokio::test]
async fn fuzzing_corrupted_payloads_with_bodies_post_handshake() {
    // tracing_subscriber::fmt::init();

    let (mut node1, mut node2) = spawn_2_fake_nodes().await;
    let write_finished = Arc::new(AtomicBool::new(false));
    let should_exit = Arc::clone(&write_finished);

    let handle = tokio::spawn(async move {
        loop {
            if should_exit.load(Ordering::Relaxed) {
                break;
            }

            let _ = node2.read_payload().await;
        }
    });

    let mut rng = thread_rng();
    let random_len: usize = rng.gen_range(1..(64 * 1024));
    let blob: Vec<u8> = rng.sample_iter(Standard).take(random_len).collect();

    let addrs: Vec<SocketAddr> = [
        "0.0.0.0:0",
        "127.0.0.1:4141",
        "192.168.1.1:14131",
        "[::1]:0",
        "[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:14131",
        "[::ffff:192.0.2.128]:4141",
    ]
    .iter()
    .map(|addr| addr.parse().unwrap())
    .collect();

    for payload in &[
        Payload::Block(blob.clone()),
        Payload::MemoryPool(vec![blob.clone(); 10]),
        Payload::SyncBlock(blob.clone()),
        Payload::Transaction(blob.clone()),
        Payload::Peers(addrs.clone()),
        Payload::Ping(thread_rng().gen()),
    ] {
        let serialized = Payload::serialize(payload).unwrap();

        for _ in 0..100 {
            let corrupted_payload = corrupt_bytes(&serialized);

            let header = MessageHeader::from(corrupted_payload.len());

            node1.write_bytes(&header.as_bytes()).await;
            node1.write_bytes(&corrupted_payload).await;
        }
    }

    write_finished.store(true, Ordering::Relaxed);
    handle.abort();
    handle.await.ok();
}

#[tokio::test]
async fn fuzzing_corrupted_payloads_with_hashes_pre_handshake() {
    // tracing_subscriber::fmt::init();

    let node_setup = TestSetup {
        consensus_setup: None,
        ..Default::default()
    };

    let node = test_node(node_setup).await;
    let node_addr = node.local_address().unwrap();

    let hashes: Vec<BlockHeaderHash> = (0u8..10).map(|i| BlockHeaderHash::new(vec![i; 32])).collect();

    for payload in &[
        Payload::GetBlocks(hashes.clone()),
        Payload::GetSync(hashes.clone()),
        Payload::Sync(hashes),
    ] {
        let serialized = Payload::serialize(payload).unwrap();

        for _ in 0..100 {
            let corrupted_payload = corrupt_bytes(&serialized);

            let header = MessageHeader::from(corrupted_payload.len());

            let mut stream = TcpStream::connect(node_addr).await.unwrap();
            let _ = stream.write_all(&header.as_bytes()).await;
            let _ = stream.write_all(&corrupted_payload).await;
        }
    }

    wait_until!(3, node.peer_book.get_active_peer_count() == 0);
}

#[tokio::test]
async fn fuzzing_corrupted_payloads_with_hashes_post_handshake() {
    // tracing_subscriber::fmt::init();

    let (mut node1, mut node2) = spawn_2_fake_nodes().await;
    let write_finished = Arc::new(AtomicBool::new(false));
    let should_exit = Arc::clone(&write_finished);

    let handle = tokio::spawn(async move {
        loop {
            if should_exit.load(Ordering::Relaxed) {
                break;
            }

            let _ = node2.read_payload().await;
        }
    });

    let hashes: Vec<BlockHeaderHash> = (0u8..10).map(|i| BlockHeaderHash::new(vec![i; 32])).collect();

    for payload in &[
        Payload::GetBlocks(hashes.clone()),
        Payload::GetSync(hashes.clone()),
        Payload::Sync(hashes),
    ] {
        let serialized = Payload::serialize(payload).unwrap();

        for _ in 0..100 {
            let corrupted_payload = corrupt_bytes(&serialized);

            let header = MessageHeader::from(corrupted_payload.len());

            node1.write_bytes(&header.as_bytes()).await;
            node1.write_bytes(&corrupted_payload).await;
        }
    }

    write_finished.store(true, Ordering::Relaxed);
    handle.abort();
    handle.await.ok();
}

#[tokio::test]
async fn connection_request_spam() {
    const NUM_ATTEMPTS: usize = 200;

    let max_peers = NUM_ATTEMPTS as u16 / 2;
    let node_setup = TestSetup {
        consensus_setup: None,
        max_peers,
        ..Default::default()
    };

    let node = test_node(node_setup).await;
    let node_addr = node.local_address().unwrap();

    let sockets = Arc::new(Mutex::new(Vec::with_capacity(NUM_ATTEMPTS)));

    for _ in 0..NUM_ATTEMPTS {
        let socks = sockets.clone();
        tokio::task::spawn(async move {
            if let Ok(socket) = TcpStream::connect(node_addr).await {
                socks.lock().await.push(socket);
            }
        });
    }

    wait_until!(3, node.peer_book.get_active_peer_count() >= max_peers as u32);

    wait_until!(
        snarkos_network::HANDSHAKE_PEER_TIMEOUT_SECS as u64 * 2,
        node.peer_book.get_active_peer_count() == 0
    );
}
