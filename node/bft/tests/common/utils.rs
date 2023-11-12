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

use crate::common::CurrentNetwork;
use snarkos_node_bft::helpers::PrimarySender;
use snarkvm::{
    ledger::narwhal::Data,
    prelude::{
        block::Transaction,
        coinbase::{ProverSolution, PuzzleCommitment},
        Field,
        Network,
        TestRng,
        Uniform,
    },
};

use std::time::Duration;

use ::bytes::Bytes;
use rand::Rng;
use tokio::{sync::oneshot, task, task::JoinHandle, time::sleep};
use tracing::*;
use tracing_subscriber::{
    layer::{Layer, SubscriberExt},
    util::SubscriberInitExt,
};

/// Initializes the logger.
#[allow(dead_code)]
pub fn initialize_logger(verbosity: u8) {
    match verbosity {
        0 => std::env::set_var("RUST_LOG", "info"),
        1 => std::env::set_var("RUST_LOG", "debug"),
        2..=4 => std::env::set_var("RUST_LOG", "trace"),
        _ => std::env::set_var("RUST_LOG", "info"),
    };

    // Filter out undesirable logs. (unfortunately EnvFilter cannot be cloned)
    let [filter] = std::array::from_fn(|_| {
        let filter = tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("mio=off".parse().unwrap())
            .add_directive("tokio_util=off".parse().unwrap())
            .add_directive("hyper=off".parse().unwrap())
            .add_directive("reqwest=off".parse().unwrap())
            .add_directive("want=off".parse().unwrap())
            .add_directive("warp=off".parse().unwrap());

        if verbosity > 3 {
            filter.add_directive("snarkos_node_tcp=trace".parse().unwrap())
        } else {
            filter.add_directive("snarkos_node_tcp=off".parse().unwrap())
        }
    });

    // Initialize tracing.
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::Layer::default().with_target(verbosity > 2).with_filter(filter))
        .try_init();
}

/// Fires *fake* unconfirmed solutions at the node.
pub fn fire_unconfirmed_solutions(
    sender: &PrimarySender<CurrentNetwork>,
    node_id: u16,
    interval_ms: u64,
) -> JoinHandle<()> {
    let tx_unconfirmed_solution = sender.tx_unconfirmed_solution.clone();
    tokio::task::spawn(async move {
        // This RNG samples the *same* fake solutions for all nodes.
        let mut shared_rng = TestRng::fixed(123456789);
        // This RNG samples *different* fake solutions for each node.
        let mut unique_rng = TestRng::fixed(node_id as u64);

        // A closure to generate a commitment and solution.
        async fn sample(mut rng: impl Rng) -> (PuzzleCommitment<CurrentNetwork>, Data<ProverSolution<CurrentNetwork>>) {
            // Sample a random fake puzzle commitment.
            // TODO (howardwu): Use a mutex to bring in the real 'proof target' and change this sampling to a while loop.
            let affine = rng.gen();
            let commitment =
                task::spawn_blocking(move || PuzzleCommitment::<CurrentNetwork>::from_g1_affine(affine)).await.unwrap();
            // Sample random fake solution bytes.
            let mut vec = vec![0u8; 1024];
            rng.fill_bytes(&mut vec);
            let solution = Data::Buffer(Bytes::from(vec));
            // Return the ID and solution.
            (commitment, solution)
        }

        // Initialize a counter.
        let mut counter = 0;

        loop {
            // Sample a random fake puzzle commitment and solution.
            let (commitment, solution) =
                if counter % 2 == 0 { sample(&mut shared_rng).await } else { sample(&mut unique_rng).await };
            // Initialize a callback sender and receiver.
            let (callback, callback_receiver) = oneshot::channel();
            // Send the fake solution.
            if let Err(e) = tx_unconfirmed_solution.send((commitment, solution, callback)).await {
                error!("Failed to send unconfirmed solution: {e}");
            }
            let _ = callback_receiver.await;
            // Increment the counter.
            counter += 1;
            // Sleep briefly.
            sleep(Duration::from_millis(interval_ms)).await;
        }
    })
}

/// Fires *fake* unconfirmed transactions at the node.
pub fn fire_unconfirmed_transactions(
    sender: &PrimarySender<CurrentNetwork>,
    node_id: u16,
    interval_ms: u64,
) -> JoinHandle<()> {
    let tx_unconfirmed_transaction = sender.tx_unconfirmed_transaction.clone();
    tokio::task::spawn(async move {
        // This RNG samples the *same* fake transactions for all nodes.
        let mut shared_rng = TestRng::fixed(123456789);
        // This RNG samples *different* fake transactions for each node.
        let mut unique_rng = TestRng::fixed(node_id as u64);

        // A closure to generate an ID and transaction.
        fn sample(
            mut rng: impl Rng,
        ) -> (<CurrentNetwork as Network>::TransactionID, Data<Transaction<CurrentNetwork>>) {
            // Sample a random fake transaction ID.
            let id = Field::<CurrentNetwork>::rand(&mut rng).into();
            // Sample random fake transaction bytes.
            let mut vec = vec![0u8; 1024];
            rng.fill_bytes(&mut vec);
            let transaction = Data::Buffer(Bytes::from(vec));
            // Return the ID and transaction.
            (id, transaction)
        }

        // Initialize a counter.
        let mut counter = 0;

        loop {
            // Sample a random fake transaction ID and transaction.
            let (id, transaction) = if counter % 2 == 0 { sample(&mut shared_rng) } else { sample(&mut unique_rng) };
            // Initialize a callback sender and receiver.
            let (callback, callback_receiver) = oneshot::channel();
            // Send the fake transaction.
            if let Err(e) = tx_unconfirmed_transaction.send((id, transaction, callback)).await {
                error!("Failed to send unconfirmed transaction: {e}");
            }
            let _ = callback_receiver.await;
            // Increment the counter.
            counter += 1;
            // Sleep briefly.
            sleep(Duration::from_millis(interval_ms)).await;
        }
    })
}
