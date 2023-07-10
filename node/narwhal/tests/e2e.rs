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

use crate::common::{
    primary::{initiate_connections, log_connections, start_n_primaries},
    utils::{fire_unconfirmed_solutions, fire_unconfirmed_transactions},
};
use snarkos_node_narwhal::MAX_BATCH_DELAY;

#[tokio::test]
#[ignore = "Long-running e2e test"]
async fn test_state_coherence() {
    // crate::common::utils::initialize_logger(0);

    const N: u16 = 4;
    let primaries = start_n_primaries(N).await;
    initiate_connections(&primaries).await;
    log_connections(&primaries);

    // Start the tx cannons for each primary.
    for (id, primary) in primaries {
        let sender = primary.1;
        // Fire unconfirmed solutions.
        fire_unconfirmed_solutions(&sender, id);
        // Fire unconfirmed transactions.
        fire_unconfirmed_transactions(&sender, id);
    }

    // TODO(nkls): the easiest would be to assert on the anchor or bullshark's output, once
    // implemented.

    // std::future::pending::<()>().await;
}

#[tokio::test]
async fn test_quorum_threshold() {
    // crate::common::utils::initialize_logger(0);

    // 1. Start N nodes but don't connect them.
    const N: u16 = 4;
    let primaries = start_n_primaries(N).await;
    log_connections(&primaries);

    // Check each node is at round 1 (0 is genesis).
    for (primary, _sender) in primaries.values() {
        assert_eq!(primary.current_round(), 1);
    }

    // 2. Start the cannon for node 0.
    {
        let (_primary_0, sender_0) = &primaries.get(&0).unwrap();
        // Fire unconfirmed solutions.
        fire_unconfirmed_solutions(sender_0, 0);
        // Fire unconfirmed transactions.
        fire_unconfirmed_transactions(sender_0, 0);
    }

    tokio::time::sleep(std::time::Duration::from_millis(MAX_BATCH_DELAY * 2)).await;

    // Check each node is still at round 1.
    for (primary, _sender) in primaries.values() {
        assert_eq!(primary.current_round(), 1);
    }

    // 3. Connect the first two nodes and start the tx cannon for the second node.
    {
        let (primary_0, _sender_0) = &primaries.get(&0).unwrap();
        let (primary_1, _sender_1) = &primaries.get(&1).unwrap();

        // Connect node 0 to node 1.
        let ip = primary_1.gateway().local_ip();
        primary_0.gateway().connect(ip);
        // Give the connection time to be established.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Fire unconfirmed solutions.
        fire_unconfirmed_solutions(_sender_1, 1);
        // Fire unconfirmed transactions.
        fire_unconfirmed_transactions(_sender_1, 1);
    }

    tokio::time::sleep(std::time::Duration::from_millis(MAX_BATCH_DELAY * 2)).await;

    // Check each node is still at round 1.
    for (primary, _sender) in primaries.values() {
        assert_eq!(primary.current_round(), 1);
    }

    // 4. Connect the third node and start the tx cannon for it.
    {
        let (primary_0, _sender_0) = &primaries.get(&0).unwrap();
        let (primary_1, _sender_1) = &primaries.get(&1).unwrap();
        let (primary_2, _sender_2) = &primaries.get(&2).unwrap();

        // Connect node 0 and 1 to node 2.
        let ip = primary_2.gateway().local_ip();
        primary_0.gateway().connect(ip);
        primary_1.gateway().connect(ip);
        // Give the connection time to be established.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Fire unconfirmed solutions.
        fire_unconfirmed_solutions(_sender_2, 2);
        // Fire unconfirmed transactions.
        fire_unconfirmed_transactions(_sender_2, 2);
    }

    // Check the nodes reach quorum and advance through the rounds.
    deadline::deadline!(std::time::Duration::from_secs(20), move || {
        let (primary_0, _sender_0) = &primaries.get(&0).unwrap();
        let (primary_1, _sender_1) = &primaries.get(&1).unwrap();
        let (primary_2, _sender_2) = &primaries.get(&2).unwrap();

        const NUM_ROUNDS: u64 = 4;
        primary_0.current_round() > NUM_ROUNDS
            && primary_1.current_round() > NUM_ROUNDS
            && primary_2.current_round() > NUM_ROUNDS
    });
}
