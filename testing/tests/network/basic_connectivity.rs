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
use snarkos_testing::{ClientNode, TestNode, MAXIMUM_NUMBER_OF_PEERS};

use pea2pea::Pea2Pea;
use std::sync::{
    atomic::{AtomicU8, Ordering::*},
    Arc,
};
use tokio::task;

#[ignore]
#[tokio::test]
async fn client_nodes_can_connect_to_each_other() {
    // Start 2 snarkOS nodes.
    let client_node1 = ClientNode::default().await;
    let client_node2 = ClientNode::default().await;

    // Connect one to the other.
    client_node1.connect(client_node2.local_addr()).await.unwrap();
}

#[ignore]
#[tokio::test]
async fn test_nodes_can_connect_to_each_other() {
    // Start 2 test nodes.
    let test_node0 = TestNode::default().await;
    let test_node1 = TestNode::default().await;

    // Ensure that the nodes have no active connections.
    assert!(test_node0.node().num_connected() == 0 && test_node1.node().num_connected() == 0);

    // Connect one to the other, performing the snarkOS handshake.
    let test_node0_addr = test_node0.node().listening_addr().unwrap();
    test_node1.node().connect(test_node0_addr).await.unwrap();

    // Ensure that both nodes have an active connection now.
    assert!(test_node0.node().num_connected() == 1 && test_node1.node().num_connected() == 1);
}

#[ignore]
#[tokio::test]
async fn handshake_as_initiator_works() {
    // Start a test node.
    let test_node = TestNode::default().await;
    let test_node_addr = test_node.node().listening_addr().unwrap();

    // Start a snarkOS node.
    let client_node = ClientNode::default().await;

    // Connect the snarkOS node to the test node.
    client_node.connect(test_node_addr).await.unwrap();

    // Double-check with the test node.
    // note: the small wait is due to the handshake responder (test node) finishing
    // the connection process a bit later than the initiator (snarkOs node).
    wait_until!(1, test_node.node().num_connected() == 1);
}

#[ignore]
#[tokio::test]
async fn handshake_as_responder_works() {
    // Start a test node.
    let test_node = TestNode::default().await;

    // Start a snarkOS node.
    let client_node = ClientNode::default().await;

    // The test node should be able to connect to the snarkOS node.
    test_node.node().connect(client_node.local_addr()).await.unwrap();

    // Double-check with the snarkOS node.
    assert!(client_node.connected_peers().await.len() == 1)
}

#[ignore]
#[tokio::test]
async fn node_cant_connect_to_itself() {
    // Start a snarkOS node.
    let client_node = ClientNode::default().await;

    // Ensure it can't connect to itself
    assert!(client_node.connect(client_node.local_addr()).await.is_err());
}

#[ignore]
#[tokio::test]
async fn node_cant_connect_to_another_twice() {
    // Start a test node.
    let test_node = TestNode::default().await;
    let test_node_addr = test_node.node().listening_addr().unwrap();

    // Start a snarkOS node.
    let client_node = ClientNode::default().await;

    // Connect the snarkOS node to the test node.
    client_node.connect(test_node_addr).await.unwrap();

    // The second connection attempt should fail.
    assert!(client_node.connect(test_node_addr).await.is_err());
}

#[ignore]
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
    let client_node = ClientNode::default().await;

    // Register the snarkOS node address and prepare a connection error counter.
    let client_node_addr = client_node.local_addr();
    let error_count = Arc::new(AtomicU8::new(0));

    // Concurrently connect to the snarkOS node, attempting to bypass the nonce uniqueness rule.
    for test_node in &test_nodes {
        let test_node = test_node.clone();
        let error_count = error_count.clone();

        task::spawn(async move {
            if test_node.node().connect(client_node_addr).await.is_err() {
                error_count.fetch_add(1, Relaxed);
            }
        });
    }

    // Ensure that only a single connection was successful.
    // note: counting errors instead of a single success ensures that all the attempts were concluded.
    wait_until!(5, error_count.load(Relaxed) == NUM_CONCURRENT_ATTEMPTS - 1);
}

#[ignore]
#[tokio::test]
async fn connection_limits_are_obeyed() {
    // Start a snarkOS node.
    let client_node = ClientNode::default().await;

    // Start the maximum number of test nodes the snarkOS node is permitted to connect to at once.
    let mut test_nodes = Vec::with_capacity(MAXIMUM_NUMBER_OF_PEERS);
    for _ in 0..MAXIMUM_NUMBER_OF_PEERS {
        test_nodes.push(TestNode::default().await);
    }

    // All the test nodes should be able to connect to the snarkOS node.
    for test_node in &test_nodes {
        test_node.node().connect(client_node.local_addr()).await.unwrap();
    }

    // Create one additional test node.
    let extra_test_node = TestNode::default().await;
    let extra_test_node_addr = extra_test_node.node().listening_addr().unwrap();

    // Assert that snarkOS can't connect to it.
    assert!(client_node.connect(extra_test_node_addr).await.is_err());

    // Assert that the test node can't connect to the snarkOS node either.
    assert!(extra_test_node.node().connect(client_node.local_addr()).await.is_err());
}

#[ignore]
#[tokio::test]
async fn peer_accounting_works() {
    // Start a snarkOS node.
    let client_node = ClientNode::default().await;

    // Start a test node.
    let test_node = TestNode::default().await;
    let test_node_addr = test_node.node().listening_addr().unwrap();

    // Double-check that the initial list of peers is empty.
    assert!(client_node.connected_peers().await.is_empty());

    // Perform the connect+disconnect routine a few fimes.
    for _ in 0..3 {
        // Connect the snarkOS node to the test node.
        client_node.connect(test_node_addr).await.unwrap();

        // Verify that the list of peers is not empty anymore.
        assert!(client_node.connected_peers().await.len() == 1);

        // The test node disconnects from the snarkOS node.
        wait_until!(1, test_node.node().num_connected() == 1);
        let client_node_addr = test_node.node().connected_addrs()[0];
        assert!(test_node.node().disconnect(client_node_addr).await);

        // The list of snarkOS peers should be empty again.
        wait_until!(1, client_node.connected_peers().await.is_empty());

        // The snarkOS node should not attempt to connect on its own.
        client_node.reset_known_peers().await;
    }
}
