// Copyright 2024 Aleo Network Foundation
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod common;
use common::*;

use snarkos_node_tcp::{protocols::Handshake, P2P};

use core::time::Duration;
use deadline::deadline;

#[tokio::test]
async fn test_connect_without_handshake() {
    // Create 2 routers.
    let node0 = validator(0, 2, &[], true).await;
    let node1 = client(0, 2).await;
    assert_eq!(node0.number_of_connected_peers(), 0);
    assert_eq!(node1.number_of_connected_peers(), 0);

    // Start listening.
    node0.tcp().enable_listener().await.unwrap();
    node1.tcp().enable_listener().await.unwrap();

    {
        // Connect node0 to node1.
        node0.connect(node1.local_ip());
        // Sleep briefly.
        tokio::time::sleep(Duration::from_millis(100)).await;

        print_tcp!(node0);
        print_tcp!(node1);

        assert_eq!(node0.tcp().num_connected(), 1);
        assert_eq!(node0.tcp().num_connecting(), 0);
        assert_eq!(node1.tcp().num_connected(), 1);
        assert_eq!(node1.tcp().num_connecting(), 0);
    }
    {
        // Connect node0 from node1 again.
        node0.connect(node1.local_ip());
        // Sleep briefly.
        tokio::time::sleep(Duration::from_millis(100)).await;

        print_tcp!(node0);
        print_tcp!(node1);

        assert_eq!(node0.tcp().num_connected(), 1);
        assert_eq!(node0.tcp().num_connecting(), 0);
        assert_eq!(node1.tcp().num_connected(), 1);
        assert_eq!(node1.tcp().num_connecting(), 0);
    }
    {
        // Connect node1 from node0.
        node1.connect(node0.local_ip());
        // Sleep briefly.
        tokio::time::sleep(Duration::from_millis(100)).await;

        print_tcp!(node0);
        print_tcp!(node1);

        assert_eq!(node0.tcp().num_connected(), 2); // node0 has no way of deduping the connection.
        assert_eq!(node0.tcp().num_connecting(), 0);
        assert_eq!(node1.tcp().num_connected(), 2); // node1 has no way of deduping the connection.
        assert_eq!(node1.tcp().num_connecting(), 0);
    }
}

#[tokio::test]
async fn test_connect_with_handshake() {
    // Create 2 routers.
    let node0 = validator(0, 2, &[], true).await;
    let node1 = client(0, 2).await;
    assert_eq!(node0.number_of_connected_peers(), 0);
    assert_eq!(node1.number_of_connected_peers(), 0);

    // Enable handshake protocol.
    node0.enable_handshake().await;
    node1.enable_handshake().await;

    // Start listening.
    node0.tcp().enable_listener().await.unwrap();
    node1.tcp().enable_listener().await.unwrap();

    {
        // Connect node0 to node1.
        node0.connect(node1.local_ip());
        // Await for node1 to be connected.
        let node0_ip = node0.local_ip();
        let node1_ = node1.clone();
        deadline!(Duration::from_secs(5), move || { node1_.is_connected(&node0_ip) });

        print_tcp!(node0);
        print_tcp!(node1);

        // Check the TCP level.
        assert_eq!(node0.tcp().num_connected(), 1);
        assert_eq!(node0.tcp().num_connecting(), 0);
        assert_eq!(node1.tcp().num_connected(), 1);
        assert_eq!(node1.tcp().num_connecting(), 0);

        // Check the router level.
        assert_eq!(node0.number_of_connected_peers(), 1);
        assert_eq!(node1.number_of_connected_peers(), 1);
    }
    {
        // Connect node0 to node1 again.
        node0.connect(node1.local_ip());
        // Await for node1 to be connected.
        let node0_ip = node0.local_ip();
        let node1_ = node1.clone();
        deadline!(Duration::from_secs(5), move || { node1_.is_connected(&node0_ip) });

        print_tcp!(node0);
        print_tcp!(node1);

        // Check the TCP level.
        assert_eq!(node0.tcp().num_connected(), 1);
        assert_eq!(node0.tcp().num_connecting(), 0);
        assert_eq!(node1.tcp().num_connected(), 1);
        assert_eq!(node1.tcp().num_connecting(), 0);

        // Check the router level.
        assert_eq!(node0.number_of_connected_peers(), 1);
        assert_eq!(node1.number_of_connected_peers(), 1);
    }
    {
        // Connect node1 to node0.
        node1.connect(node0.local_ip());
        // Await for node0 to be connected.
        let node1_ip = node1.local_ip();
        let node0_ = node0.clone();
        deadline!(Duration::from_secs(5), move || { node0_.is_connected(&node1_ip) });

        print_tcp!(node0);
        print_tcp!(node1);

        // Check the TCP level.
        assert_eq!(node0.tcp().num_connected(), 1);
        assert_eq!(node0.tcp().num_connecting(), 0);
        assert_eq!(node1.tcp().num_connected(), 1);
        assert_eq!(node1.tcp().num_connecting(), 0);

        // Check the router level.
        assert_eq!(node0.number_of_connected_peers(), 1);
        assert_eq!(node1.number_of_connected_peers(), 1);
    }
}

#[tokio::test]
async fn test_validator_connection() {
    // Create first router and start listening.
    let node0 = validator(0, 2, &[], false).await;
    assert_eq!(node0.number_of_connected_peers(), 0);
    node0.enable_handshake().await;
    node0.tcp().enable_listener().await.unwrap();

    // Get the local IP address from the first router.
    let addr0 = node0.local_ip();

    // Create second router, trusting the first router, and start listening.
    let node1 = validator(0, 2, &[addr0], false).await;
    assert_eq!(node1.number_of_connected_peers(), 0);
    node1.enable_handshake().await;
    node1.tcp().enable_listener().await.unwrap();

    {
        // Connect node0 to node1.
        node0.connect(node1.local_ip());
        // Await for node1 to be connected.
        let node0_ip = node0.local_ip();
        let node1_ = node1.clone();
        deadline!(Duration::from_secs(5), move || { node1_.is_connected(&node0_ip) });

        print_tcp!(node0);
        print_tcp!(node1);

        // Check the TCP level - connection was accepted.
        assert_eq!(node0.tcp().num_connected(), 1);
        assert_eq!(node1.tcp().num_connected(), 1);

        // Check the router level - connection was accepted.
        assert_eq!(node0.number_of_connected_peers(), 1);
        assert_eq!(node1.number_of_connected_peers(), 1);

        // Disconnect the nodes.
        node0.disconnect(node1.local_ip());
        node1.disconnect(node0.local_ip());

        // Await for node1 and node0 to be disconnected.
        let node1_ = node1.clone();
        let node0_ = node0.clone();
        deadline!(Duration::from_secs(5), move || {
            !node1_.is_connected(&node0_.local_ip()) && !node0_.is_connected(&node1_.local_ip())
        });

        // Connect node1 to node0.
        let Ok(res) = node1.connect(node0.local_ip()).unwrap().await else {
            panic!("Connection failed for the wrong reasons.");
        };
        assert!(!res, "Connection was accepted when it should not have been.");

        // Check the TCP level - connection was not accepted.
        assert_eq!(node0.tcp().num_connected(), 0);
        assert_eq!(node1.tcp().num_connected(), 0);

        // Check the router level - connection was not accepted.
        assert_eq!(node0.number_of_connected_peers(), 0);
        assert_eq!(node1.number_of_connected_peers(), 0);
    }
}

#[ignore]
#[tokio::test]
async fn test_connect_simultaneously_with_handshake() {
    // Create 2 routers.
    let node0 = validator(0, 2, &[], true).await;
    let node1 = client(0, 2).await;
    assert_eq!(node0.number_of_connected_peers(), 0);
    assert_eq!(node1.number_of_connected_peers(), 0);

    // Enable handshake protocol.
    node0.enable_handshake().await;
    node1.enable_handshake().await;

    {
        // Connect node0 to node1.
        node0.connect(node1.local_ip());
        // Connect node1 to node0.
        node1.connect(node0.local_ip());
        // Sleep briefly.
        tokio::time::sleep(Duration::from_millis(100)).await;

        print_tcp!(node0);
        print_tcp!(node1);

        // Check the TCP level.
        assert_eq!(node0.tcp().num_connected(), 1);
        assert_eq!(node0.tcp().num_connecting(), 0);
        assert_eq!(node1.tcp().num_connected(), 1);
        assert_eq!(node1.tcp().num_connecting(), 0);

        // Check the router level.
        assert_eq!(node0.number_of_connected_peers(), 1);
        assert_eq!(node1.number_of_connected_peers(), 1);
    }
}
