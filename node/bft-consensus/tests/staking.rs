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

use std::time::Duration;

use bytes::Bytes;
use narwhal_types::TransactionProto;
use rand::prelude::{IteratorRandom, Rng};
use snarkvm::prelude::TestRng;
use tokio::time::sleep;

mod common;

use crate::common::{StakeChange, Transaction};
use common::generate_running_consensus_instances;

#[tokio::test(flavor = "multi_thread")]
async fn staking() {
    // Configure the primary-related variables.
    const NUM_PRIMARIES: usize = 5;
    const PRIMARY_STAKE: u64 = 1;

    // Configure the number of stake changes to perform.
    const NUM_STAKE_CHANGES: usize = 100;

    let (state, running_consensus_instances) = generate_running_consensus_instances(NUM_PRIMARIES, PRIMARY_STAKE).await;

    // Create transaction clients; any instance can be used to do that.
    let mut tx_clients = running_consensus_instances[0].spawn_tx_clients();

    // Use a deterministic Rng.
    let mut rng = TestRng::default();

    for i in 0..NUM_STAKE_CHANGES {
        // 1. Generate a stake change for a random authority.
        let (pub_key, _authority) = state.committee.load().authorities.clone().into_iter().choose(&mut rng).unwrap();

        // Generate a random stake change.
        let change = rng.gen_range(-10..=10);
        // Create a stake change transaction.
        let stake_tx = Transaction::StakeChange(StakeChange { id: i as u64, pub_key: pub_key.clone(), change });

        // 2. Send the transactions to a random number of BFT workers at a time.
        // Randomize the number of worker recipients.
        let n_recipients: usize = rng.gen_range(1..=tx_clients.len());

        let transaction: Bytes = bincode::serialize(&stake_tx).unwrap().into();
        let tx = TransactionProto { transaction };

        // Submit the transaction to the chosen workers.
        for tx_client in tx_clients.iter_mut().choose_multiple(&mut rng, n_recipients) {
            tx_client.submit_transaction(tx.clone()).await.unwrap();
        }
    }

    // Wait for a while to allow the transfers to be processed.
    sleep(Duration::from_secs(10)).await;

    // 3. Check the state matches across the network.
    let first_state = running_consensus_instances[0].state.committee.load_full();
    for state in running_consensus_instances.iter().skip(1).map(|rci| &rci.state) {
        assert_eq!(first_state, state.committee.load_full());
    }
}
