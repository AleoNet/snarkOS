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

use std::{sync::atomic::Ordering, time::Duration};

use bytes::Bytes;
use deadline::deadline;
use narwhal_types::TransactionProto;
use rand::prelude::{IteratorRandom, Rng};
use snarkvm::prelude::TestRng;
use tokio::time::{sleep, timeout};

mod common;

use common::generate_running_consensus_instances;

// Makes sure that all the primaries have identical state after
// having processed a range of transactions using the consensus.
#[tokio::test(flavor = "multi_thread")]
async fn verify_state_coherence() {
    // Configure the primary-related variables.
    const NUM_PRIMARIES: usize = 5;
    const PRIMARY_STAKE: u64 = 1;

    // Configure the transactions.
    const NUM_TRANSACTIONS: usize = 100;

    // Set up the base state.
    let (state, running_consensus_instances) = generate_running_consensus_instances(NUM_PRIMARIES, PRIMARY_STAKE).await;

    // Create transaction clients; any instance can be used to do that.
    let mut tx_clients = running_consensus_instances[0].spawn_tx_clients();

    // Use a deterministic Rng for transaction generation.
    let mut rng = TestRng::default();

    // Generate random transactions.
    let transfers = state.generate_random_transfers(NUM_TRANSACTIONS, &mut rng);

    // Send the transactions to a random number of BFT workers at a time.
    for transfer in transfers {
        // Randomize the number of worker recipients.
        let n_recipients: usize = rng.gen_range(1..=tx_clients.len());

        let transaction: Bytes = bincode::serialize(&transfer).unwrap().into();
        let tx = TransactionProto { transaction };

        // Submit the transaction to the chosen workers.
        for tx_client in tx_clients.iter_mut().choose_multiple(&mut rng, n_recipients) {
            tx_client.submit_transaction(tx.clone()).await.unwrap();
        }
    }

    // Wait for a while to allow the transfers to be processed.
    sleep(Duration::from_secs(3)).await;

    // Check that all the states match.
    let first_state = &running_consensus_instances[0].state;
    for state in running_consensus_instances.iter().skip(1).map(|rci| &rci.state) {
        assert_eq!(first_state, state);
    }
}

// Ensures that a committee can survive the expected number of member failures,
// and that it ceases to function with a single additional failure.
#[tokio::test(flavor = "multi_thread")]
async fn primary_failures() {
    // Configure the primary-related variables.
    const NUM_PRIMARIES: usize = 5;
    // TODO: extend the test to different stakes
    const PRIMARY_STAKE: u64 = 1;

    // Calculate the maximum allowed number of primary failures.
    // note: it's the number of primaries minus the quorum
    const MAX_FAILURES: usize = NUM_PRIMARIES - (2 * NUM_PRIMARIES / 3 + 1);

    // Configure the transaction counts.
    const NUM_TXS_PER_ITER: usize = 5;
    const NUM_TRANSACTIONS: usize = (MAX_FAILURES + 2) * NUM_TXS_PER_ITER;

    // Set up the base state.
    let (state, mut running_consensus_instances) =
        generate_running_consensus_instances(NUM_PRIMARIES, PRIMARY_STAKE).await;

    // Use a deterministic Rng for transaction generation.
    let mut rng = TestRng::default();

    // Generate random transactions.
    let mut transfers = state.generate_random_transfers(NUM_TRANSACTIONS, &mut rng);

    // Create transaction clients; any instance can be used to do that.
    let mut tx_clients = running_consensus_instances[0].spawn_tx_clients();

    // We stop when the consensus ceases to function (the timeout).
    for i in 0.. {
        // Save the number of processed transactions before the next batch is distributed.
        // note: in the first iteration it's zero for everyone, and later on it's guaranteed
        // to be coherent due to us waiting for the numbers to be aligned for everyone.
        let tx_count_before_batch = running_consensus_instances[0].state.processed_txs.load(Ordering::SeqCst);

        // Prepare a batch of transactions to be sent to the workers.
        let tx_batch = transfers
            .drain(..NUM_TXS_PER_ITER)
            .map(|tx| {
                let transaction: Bytes = bincode::serialize(&tx).unwrap().into();
                TransactionProto { transaction }
            })
            .collect::<Vec<_>>();

        // Submit the transactions to the workers.
        let mut clients = tx_clients.clone();
        if timeout(
            Duration::from_secs(3),
            tokio::spawn(async move {
                for tx in tx_batch {
                    for tx_client in &mut clients {
                        tx_client.submit_transaction(tx.clone()).await.unwrap();
                    }
                }
            }),
        )
        .await
        .is_err()
        {
            if i == MAX_FAILURES + 1 {
                // Once the maximum number of primary failures is breached, a timeout is expected.
                break;
            } else {
                panic!("Unexpected transaction transmission timeout at {i} failures instead of {}", MAX_FAILURES + 1);
            }
        }

        // Wait for the transfers to be processed by everyone.
        let states = running_consensus_instances.iter().map(|rci| rci.state.clone()).collect::<Vec<_>>();
        // Use a generous timeout in case many primaries are tested.
        deadline!(Duration::from_secs(10), move || {
            let mut states = states.iter();
            let first_tx_count = states.next().unwrap().processed_txs.load(Ordering::SeqCst);

            if first_tx_count == tx_count_before_batch {
                return false;
            }

            states.map(|state| state.processed_txs.load(Ordering::SeqCst)).all(|tx_count| tx_count == first_tx_count)
        });

        // Kill one of the consensus instances and shut down the corresponding transaction client.
        let instance_idx = rng.gen_range(0..NUM_PRIMARIES - i);
        let instance = running_consensus_instances.swap_remove(instance_idx);
        for worker_node in &instance.worker_nodes {
            worker_node.shutdown().await;
        }
        instance.primary_node.shutdown().await;
        drop(instance);
        // This index matches the consensus instance one due to us sorting the clients by the port.
        tx_clients.swap_remove(instance_idx);
    }
}
