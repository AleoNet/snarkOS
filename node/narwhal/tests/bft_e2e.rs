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
use deadline::deadline;
use itertools::Itertools;
use snarkos_node_narwhal::MAX_BATCH_DELAY;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "long-running e2e test"]
async fn test_state_coherence() {
    const N: u16 = 4;
    const CANNON_INTERVAL_MS: u64 = 10;

    let mut network = TestNetwork::new(TestNetworkConfig {
        num_nodes: N,
        bft: true,
        connect_all: true,
        fire_cannons: Some(CANNON_INTERVAL_MS),
        // Set this to Some(0..=4) to see the logs.
        log_level: Some(0),
        log_connections: true,
    });

    network.start().await;

    std::future::pending::<()>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_quorum_threshold() {
    // Start N nodes but don't connect them.
    const N: u16 = 4;
    const CANNON_INTERVAL_MS: u64 = 10;

    let mut network = TestNetwork::new(TestNetworkConfig {
        num_nodes: N,
        bft: true,
        connect_all: false,
        fire_cannons: None,
        // Set this to Some(0..=4) to see the logs.
        log_level: None,
        log_connections: true,
    });
    network.start().await;

    // Check each node is at round 1 (0 is genesis).
    for validators in network.validators.values() {
        assert_eq!(validators.primary.current_round(), 1);
    }

    // Start the cannons for node 0.
    network.fire_cannons_at(0, CANNON_INTERVAL_MS);

    sleep(Duration::from_millis(MAX_BATCH_DELAY * 2)).await;

    // Check each node is still at round 1.
    for validator in network.validators.values() {
        assert_eq!(validator.primary.current_round(), 1);
    }

    // Connect the first two nodes and start the cannons for node 1.
    network.connect_validators(0, 1).await;
    network.fire_cannons_at(1, CANNON_INTERVAL_MS);

    sleep(Duration::from_millis(MAX_BATCH_DELAY * 2)).await;

    // Check each node is still at round 1.
    for validator in network.validators.values() {
        assert_eq!(validator.primary.current_round(), 1);
    }

    // Connect the third node and start the cannons for it.
    network.connect_validators(0, 2).await;
    network.connect_validators(1, 2).await;
    network.fire_cannons_at(2, CANNON_INTERVAL_MS);

    // Check the nodes reach quorum and advance through the rounds.
    const TARGET_ROUND: u64 = 4;
    deadline!(Duration::from_secs(20), move || { network.is_round_reached(TARGET_ROUND) });
}

#[tokio::test(flavor = "multi_thread")]
async fn test_quorum_break() {
    // Start N nodes, connect them and start the cannons for each.
    const N: u16 = 4;
    const CANNON_INTERVAL_MS: u64 = 10;
    let mut network = TestNetwork::new(TestNetworkConfig {
        num_nodes: N,
        bft: true,
        connect_all: true,
        fire_cannons: Some(CANNON_INTERVAL_MS),
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

#[tokio::test(flavor = "multi_thread")]
async fn test_leader_election_consistency() {
    // The minimum and maximum rounds to check for leader consistency.
    // From manual experimentation, the minimum round that works is 4.
    // Starting at 0 or 2 causes assertion failures. Seems like the committee takes a few rounds to stabilize.
    const STARTING_ROUND: u64 = 4;
    const MAX_ROUND: u64 = 30;

    // Start N nodes, connect them and start the cannons for each.
    const N: u16 = 4;
    const CANNON_INTERVAL_MS: u64 = 10;
    let mut network = TestNetwork::new(TestNetworkConfig {
        num_nodes: N,
        bft: true,
        connect_all: true,
        fire_cannons: Some(CANNON_INTERVAL_MS),
        // Set this to Some(0..=4) to see the logs.
        log_level: Some(1),
        log_connections: true,
    });
    network.start().await;

    // Wait for starting round to be reached
    let cloned_network = network.clone();
    deadline!(Duration::from_secs(60), move || { cloned_network.is_round_reached(STARTING_ROUND) });

    // Check that validators agree about leaders in every even round
    for target_round in (STARTING_ROUND..=MAX_ROUND).step_by(2) {
        let cloned_network = network.clone();
        deadline!(Duration::from_secs(20), move || { cloned_network.is_round_reached(target_round) });

        // Get all leaders
        let leaders = network
            .validators
            .values()
            .flat_map(|v| v.bft.clone().map(|bft| bft.leader()))
            .flatten()
            .collect::<Vec<_>>();
        println!("Found {} validators with a leader (out of {})", leaders.len(), network.validators.values().count());

        // Assert that we have N leaders
        assert_eq!(leaders.len(), N as usize);

        // Assert that all leaders are equal
        assert!(leaders.iter().all_equal());
    }
}
