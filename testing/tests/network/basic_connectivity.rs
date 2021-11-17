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

use crate::{common::spawn_test_node_with_nonce, wait_until};

use pea2pea::Pea2Pea;
use snarkos_testing::{SnarkosNode, TestNode};
use tokio::task;

use std::sync::{
    atomic::{AtomicU8, Ordering::*},
    Arc,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_nodes_can_connect_to_each_other() {
    // Start 2 test nodes.
    let test_node0 = TestNode::default().await;
    let test_node1 = TestNode::default().await;

    // Ensure that the nodes have no active connections.
    crate::wait_until!(1, test_node0.node().num_connected() == 0 && test_node1.node().num_connected() == 0);

    // Connect one to the other, performing the snarkOS handshake.
    let test_node0_addr = test_node0.node().listening_addr().unwrap();
    assert!(test_node1.node().connect(test_node0_addr).await.is_ok());

    // Ensure that both nodes have an active connection now.
    crate::wait_until!(1, test_node0.node().num_connected() == 1 && test_node1.node().num_connected() == 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn handshake_as_initiator_works() {
    // Start a test node.
    let test_node = TestNode::default().await;

    // Start a snarkOS node.
    let test_node_addr = test_node.node().listening_addr().unwrap();
    SnarkosNode::with_args(&["--node", "0", "--connect", &test_node_addr.to_string()]).await;

    // The snarkOS node should have connected to the test node.
    crate::wait_until!(5, test_node.node().num_connected() != 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn handshake_as_responder_works() {
    // Start a test node.
    let test_node = TestNode::default().await;

    // Start a snarkOS node.
    let snarkos_node = SnarkosNode::default().await;

    // The test node should be able to connect to the snarkOS node.
    assert!(test_node.node().connect(snarkos_node.addr).await.is_ok());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "TODO: currently not possible to connect on demand"]
async fn node_cant_connect_to_itself() {}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "TODO: currently not possible to connect on demand"]
async fn node_cant_connect_to_another_twice() {}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_duplicate_connection_attempts_fail() {
    // The number of concurrent connection attempts.
    const NUM_CONCURRENT_ATTEMPTS: u8 = 5;

    // Start the test nodes, all with the same handshake nonce.
    let mut test_nodes = Vec::with_capacity(NUM_CONCURRENT_ATTEMPTS as usize);
    for _ in 0..NUM_CONCURRENT_ATTEMPTS {
        test_nodes.push(spawn_test_node_with_nonce(0).await);
    }

    // Start a snarkOS node.
    let snarkos_node = SnarkosNode::default().await;

    // Register the snarkOS node address and prepare a connection error counter.
    let snarkos_node_addr = snarkos_node.addr;
    let error_count = Arc::new(AtomicU8::new(0));

    // Concurrently connect to the snarkOS node, attempting to bypass the nonce uniqueness rule.
    for test_node in &test_nodes {
        let test_node = test_node.clone();
        let error_count = error_count.clone();

        task::spawn(async move {
            if test_node.node().connect(snarkos_node_addr).await.is_err() {
                error_count.fetch_add(1, Relaxed);
            }
        });
    }

    // Ensure that only a single connection was successful.
    // note: counting errors instead of a single success ensures that all the attempts were concluded.
    wait_until!(5, error_count.load(Relaxed) == NUM_CONCURRENT_ATTEMPTS - 1);
}
