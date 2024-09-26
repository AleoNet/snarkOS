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

#[tokio::test]
async fn test_disconnect_without_handshake() {
    // Create 2 routers.
    let node0 = validator(0, 1, &[], true).await;
    let node1 = client(0, 1).await;
    assert_eq!(node0.number_of_connected_peers(), 0);
    assert_eq!(node1.number_of_connected_peers(), 0);

    // Start listening.
    node0.tcp().enable_listener().await.unwrap();
    node1.tcp().enable_listener().await.unwrap();

    // Connect node0 to node1.
    node0.connect(node1.local_ip());
    // Sleep briefly.
    tokio::time::sleep(Duration::from_millis(200)).await;

    print_tcp!(node0);
    print_tcp!(node1);

    assert_eq!(node0.tcp().num_connected(), 1);
    assert_eq!(node0.tcp().num_connecting(), 0);
    assert_eq!(node1.tcp().num_connected(), 1);
    assert_eq!(node1.tcp().num_connecting(), 0);

    // Disconnect node0 from node1.
    // note: the lower-level disconnect call is used, as the higher-level
    // collection of connected peers is only altered during the handshake,
    // as well as the address resolver needed for the higher-level calls
    node0.tcp().disconnect(node1.local_ip()).await;
    // Sleep briefly.
    tokio::time::sleep(Duration::from_millis(100)).await;

    print_tcp!(node0);
    print_tcp!(node1);

    assert_eq!(node0.tcp().num_connected(), 0);
    assert_eq!(node0.tcp().num_connecting(), 0);
    assert_eq!(node1.tcp().num_connected(), 1); // Router 1 has no way of knowing that Router 0 disconnected.
    assert_eq!(node1.tcp().num_connecting(), 0);
}

#[tokio::test]
async fn test_disconnect_with_handshake() {
    // Create 2 routers.
    let node0 = validator(0, 1, &[], true).await;
    let node1 = client(0, 1).await;
    assert_eq!(node0.number_of_connected_peers(), 0);
    assert_eq!(node1.number_of_connected_peers(), 0);

    // Enable handshake protocol.
    node0.enable_handshake().await;
    node1.enable_handshake().await;

    // Start listening.
    node0.tcp().enable_listener().await.unwrap();
    node1.tcp().enable_listener().await.unwrap();

    // Connect node0 to node1.
    node0.connect(node1.local_ip());
    // Sleep briefly.
    tokio::time::sleep(Duration::from_millis(1000)).await;

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

    // Disconnect node0 from node1.
    node0.disconnect(node1.local_ip());
    // Sleep briefly.
    tokio::time::sleep(Duration::from_millis(100)).await;

    print_tcp!(node0);
    print_tcp!(node1);

    assert_eq!(node0.tcp().num_connected(), 0);
    assert_eq!(node0.tcp().num_connecting(), 0);
    assert_eq!(node1.tcp().num_connected(), 1); // Router 1 has no way of knowing that Router 0 disconnected.
    assert_eq!(node1.tcp().num_connecting(), 0);
}
