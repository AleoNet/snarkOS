// Copyright (C) 2019-2023 Aleo Systems Inc.
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
use crate::common::primary::{TestNetwork, TestNetworkConfig};
use snarkos_node_narwhal::MAX_BATCH_DELAY;

use std::time::Duration;

use deadline::deadline;
use tokio::time::sleep;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Long-running e2e test"]
async fn test_state_coherence() {
    const N: u16 = 4;
    let mut network = TestNetwork::new(TestNetworkConfig {
        num_nodes: N,
        connect_all: true,
        fire_cannons: true,

        // Set this to Some(0..=4) to see the logs.
        log_level: None,
        log_connections: true,
    });

    network.start().await;

    // TODO(nkls): the easiest would be to assert on the anchor or bullshark's output, once
    // implemented.

    std::future::pending::<()>().await;
}

#[tokio::test]
async fn test_quorum_threshold() {
    // Start N nodes but don't connect them.
    const N: u16 = 4;
    let mut network = TestNetwork::new(TestNetworkConfig {
        num_nodes: N,
        connect_all: false,
        fire_cannons: false,

        // Set this to Some(0..=4) to see the logs.
        log_level: None,
        log_connections: true,
    });
    network.start().await;

    // Check each node is at round 1 (0 is genesis).
    for primary in network.primaries.values() {
        assert_eq!(primary.current_round(), 1);
    }

    // Start the cannons for node 0.
    network.fire_cannons_at(0);

    sleep(Duration::from_millis(MAX_BATCH_DELAY * 2)).await;

    // Check each node is still at round 1.
    for primary in network.primaries.values() {
        assert_eq!(primary.current_round(), 1);
    }

    // Connect the first two nodes and start the cannons for node 1.
    network.connect_primaries(0, 1).await;
    network.fire_cannons_at(1);

    sleep(Duration::from_millis(MAX_BATCH_DELAY * 2)).await;

    // Check each node is still at round 1.
    for primary in network.primaries.values() {
        assert_eq!(primary.current_round(), 1);
    }

    // Connect the third node and start the cannons for it.
    network.connect_primaries(0, 2).await;
    network.connect_primaries(1, 2).await;
    network.fire_cannons_at(2);

    // Check the nodes reach quorum and advance through the rounds.
    const TARGET_ROUND: u64 = 4;
    deadline!(Duration::from_secs(20), move || { network.is_round_reached(TARGET_ROUND) });
}

#[tokio::test]
async fn test_quorum_break() {
    // Start N nodes, connect them and start the cannons for each.
    const N: u16 = 4;
    let mut network = TestNetwork::new(TestNetworkConfig {
        num_nodes: N,
        connect_all: true,
        fire_cannons: true,

        // Set this to Some(0..=4) to see the logs.
        log_level: None,
        log_connections: true,
    });
    network.start().await;

    // Check the nodes have started advancing through the rounds.
    const TARGET_ROUND: u64 = 4;
    // Note: cloning the network is fine because the primaries it wraps are `Arc`ed.
    let network_clone = network.clone();
    deadline!(Duration::from_secs(20), move || { network_clone.is_round_reached(TARGET_ROUND) });

    // Break the quorum by disconnecting two nodes.
    const NUM_NODES: u16 = 2;
    network.disconnect(NUM_NODES).await;

    // Check the nodes have stopped advancing through the rounds.
    assert!(network.is_halted().await);
}
