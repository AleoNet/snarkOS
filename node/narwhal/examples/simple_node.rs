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

#[macro_use]
extern crate tracing;

use snarkos_account::Account;
use snarkos_node_narwhal::{
    helpers::{init_primary_channels, Committee, PrimarySender, Storage},
    Primary,
    MAX_GC_ROUNDS,
    MEMORY_POOL_PORT,
};
use snarkvm::{
    ledger::narwhal::Data,
    prelude::{
        block::Transaction,
        coinbase::{ProverSolution, PuzzleCommitment},
        Field,
        Network,
        Uniform,
    },
};

use ::bytes::Bytes;
use anyhow::{bail, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_extra::response::ErasedJson;
use indexmap::IndexMap;
use parking_lot::RwLock;
use rand::{Rng, SeedableRng};
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use tracing_subscriber::{
    layer::{Layer, SubscriberExt},
    util::SubscriberInitExt,
};

type CurrentNetwork = snarkvm::prelude::Testnet3;

/**************************************************************************************************/

/// Initializes the logger.
pub fn initialize_logger(verbosity: u8) {
    match verbosity {
        0 => std::env::set_var("RUST_LOG", "info"),
        1 => std::env::set_var("RUST_LOG", "debug"),
        2 | 3 | 4 => std::env::set_var("RUST_LOG", "trace"),
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

/**************************************************************************************************/

/// Starts the primary instance.
pub async fn start_primary(
    node_id: u16,
    num_nodes: u16,
) -> Result<(Primary<CurrentNetwork>, PrimarySender<CurrentNetwork>)> {
    // Sample a account.
    let account = Account::new(&mut rand_chacha::ChaChaRng::seed_from_u64(node_id as u64))?;
    println!("\n{account}\n");

    // Initialize a map for the committee members.
    let mut members = IndexMap::with_capacity(num_nodes as usize);
    // Add the validators as members.
    for i in 0..num_nodes {
        // Sample the account.
        let account = Account::new(&mut rand_chacha::ChaChaRng::seed_from_u64(i as u64))?;
        // Add the validator.
        members.insert(account.address(), 1000);
        println!("  Validator {}: {}", i, account.address());
    }
    println!();

    // Initialize the committee.
    let committee = Arc::new(RwLock::new(Committee::<CurrentNetwork>::new(1u64, members)?));
    // Initialize the storage.
    let storage = Storage::new(MAX_GC_ROUNDS);

    // Initialize the primary channels.
    let (sender, receiver) = init_primary_channels();
    // Initialize the primary instance.
    let mut primary = Primary::<CurrentNetwork>::new(committee.clone(), storage, account, Some(node_id))?;
    // Run the primary instance.
    primary.run(sender.clone(), receiver).await?;
    // Keep the node's connections.
    keep_connections(&primary, node_id, num_nodes);
    // Handle the log connections.
    log_connections(&primary);
    // Handle OS signals.
    handle_signals(&primary);
    // Return the primary instance.
    Ok((primary, sender))
}

/// Actively try to keep the node's connections to all nodes.
fn keep_connections(primary: &Primary<CurrentNetwork>, node_id: u16, num_nodes: u16) {
    let node = primary.clone();
    tokio::task::spawn(async move {
        // Sleep briefly to ensure the other nodes are ready to connect.
        tokio::time::sleep(std::time::Duration::from_millis(100 * node_id as u64)).await;
        // Start the loop.
        loop {
            for i in 0..num_nodes {
                // Initialize the gateway IP.
                let ip = SocketAddr::from_str(&format!("127.0.0.1:{}", MEMORY_POOL_PORT + i)).unwrap();
                // Check if the node is connected.
                if i != node_id && !node.gateway().is_connected(ip) {
                    // Connect to the node.
                    node.gateway().connect(ip);
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });
}

/// Logs the node's connections.
fn log_connections(primary: &Primary<CurrentNetwork>) {
    let node = primary.clone();
    tokio::task::spawn(async move {
        loop {
            let connections = node.gateway().connected_peers().read().clone();
            info!("{} connections", connections.len());
            for connection in connections {
                debug!("  {}", connection);
            }
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });
}

/// Handles OS signals for the node to intercept and perform a clean shutdown.
/// Note: Only Ctrl-C is supported; it should work on both Unix-family systems and Windows.
fn handle_signals(primary: &Primary<CurrentNetwork>) {
    let node = primary.clone();
    tokio::task::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                node.shut_down().await;
                std::process::exit(0);
            }
            Err(error) => error!("tokio::signal::ctrl_c encountered an error: {}", error),
        }
    });
}

/**************************************************************************************************/

/// Fires *fake* unconfirmed solutions at the node.
fn fire_unconfirmed_solutions(sender: &PrimarySender<CurrentNetwork>, node_id: u16) {
    let tx_unconfirmed_solution = sender.tx_unconfirmed_solution.clone();
    tokio::task::spawn(async move {
        // This RNG samples the *same* fake solutions for all nodes.
        let mut shared_rng = rand_chacha::ChaChaRng::seed_from_u64(123456789);
        // This RNG samples *different* fake solutions for each node.
        let mut unique_rng = rand_chacha::ChaChaRng::seed_from_u64(node_id as u64);

        // A closure to generate a commitment and solution.
        fn sample(mut rng: impl Rng) -> (PuzzleCommitment<CurrentNetwork>, Data<ProverSolution<CurrentNetwork>>) {
            // Sample a random fake puzzle commitment.
            // TODO (howardwu): Use a mutex to bring in the real 'proof target' and change this sampling to a while loop.
            let commitment = PuzzleCommitment::<CurrentNetwork>::from_g1_affine(rng.gen());
            // Sample random fake solution bytes.
            let solution = Data::Buffer(Bytes::from((0..1024).map(|_| rng.gen::<u8>()).collect::<Vec<_>>()));
            // Return the ID and solution.
            (commitment, solution)
        }

        // Initialize a counter.
        let mut counter = 0;

        loop {
            // Sample a random fake puzzle commitment and solution.
            let (commitment, solution) =
                if counter % 2 == 0 { sample(&mut shared_rng) } else { sample(&mut unique_rng) };
            // Send the fake solution.
            if let Err(e) = tx_unconfirmed_solution.send((commitment, solution)).await {
                error!("Failed to send unconfirmed solution: {e}");
            }
            // Increment the counter.
            counter += 1;
            // Sleep briefly.
            tokio::time::sleep(std::time::Duration::from_millis(450)).await;
        }
    });
}

/// Fires *fake* unconfirmed transactions at the node.
fn fire_unconfirmed_transactions(sender: &PrimarySender<CurrentNetwork>, node_id: u16) {
    let tx_unconfirmed_transaction = sender.tx_unconfirmed_transaction.clone();
    tokio::task::spawn(async move {
        // This RNG samples the *same* fake transactions for all nodes.
        let mut shared_rng = rand_chacha::ChaChaRng::seed_from_u64(123456789);
        // This RNG samples *different* fake transactions for each node.
        let mut unique_rng = rand_chacha::ChaChaRng::seed_from_u64(node_id as u64);

        // A closure to generate an ID and transaction.
        fn sample(
            mut rng: impl Rng,
        ) -> (<CurrentNetwork as Network>::TransactionID, Data<Transaction<CurrentNetwork>>) {
            // Sample a random fake transaction ID.
            let id = Field::<CurrentNetwork>::rand(&mut rng).into();
            // Sample random fake transaction bytes.
            let transaction = Data::Buffer(Bytes::from((0..1024).map(|_| rng.gen::<u8>()).collect::<Vec<_>>()));
            // Return the ID and transaction.
            (id, transaction)
        }

        // Initialize a counter.
        let mut counter = 0;

        loop {
            // Sample a random fake transaction ID and transaction.
            let (id, transaction) = if counter % 2 == 0 { sample(&mut shared_rng) } else { sample(&mut unique_rng) };
            // Send the fake transaction.
            if let Err(e) = tx_unconfirmed_transaction.send((id, transaction)).await {
                error!("Failed to send unconfirmed transaction: {e}");
            }
            // Increment the counter.
            counter += 1;
            // Sleep briefly.
            tokio::time::sleep(std::time::Duration::from_millis(450)).await;
        }
    });
}

/**************************************************************************************************/

/// An enum of error handlers for the REST API server.
pub struct RestError(pub String);

impl IntoResponse for RestError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Something went wrong: {}", self.0)).into_response()
    }
}

impl From<anyhow::Error> for RestError {
    fn from(err: anyhow::Error) -> Self {
        Self(err.to_string())
    }
}

/// Returns the current round.
async fn get_current_round(State(primary): State<Primary<CurrentNetwork>>) -> Result<ErasedJson, RestError> {
    Ok(ErasedJson::pretty(primary.committee().read().round()))
}

/// Returns the certificates for the given round.
async fn get_certificates_for_round(
    State(primary): State<Primary<CurrentNetwork>>,
    Path(round): Path<u64>,
) -> Result<ErasedJson, RestError> {
    Ok(ErasedJson::pretty(primary.storage().get_certificates_for_round(round)))
}

/// Starts up a local server for monitoring the node.
async fn start_server(primary: Primary<CurrentNetwork>, node_id: u16) {
    // Initialize the routes.
    let router = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/round/current", get(get_current_round))
        .route("/certificates/:round", get(get_certificates_for_round))
        // Pass in the `Primary` to access state.
        .with_state(primary);

    // Construct the IP address and port.
    let addr = format!("127.0.0.1:{}", 3000 + node_id);

    // Run the server.
    info!("Starting the server at '{addr}'...");
    axum::Server::bind(&addr.parse().unwrap())
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

/**************************************************************************************************/

#[tokio::main]
async fn main() -> Result<()> {
    initialize_logger(1);

    // Retrieve the command-line arguments.
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        bail!("Please provide a command.")
    }

    // Parse the node ID.
    let node_id = u16::from_str(&args[1])?;
    // Parse the number of nodes.
    let num_nodes = u16::from_str(&args[2])?;

    // Start the primary instance.
    let (primary, sender) = start_primary(node_id, num_nodes).await?;

    // Fire unconfirmed solutions.
    fire_unconfirmed_solutions(&sender, node_id);
    // Fire unconfirmed transactions.
    fire_unconfirmed_transactions(&sender, node_id);

    println!("Hello, world!");

    // Start the monitoring server.
    start_server(primary, node_id).await;
    // // Note: Do not move this.
    // std::future::pending::<()>().await;
    Ok(())
}
