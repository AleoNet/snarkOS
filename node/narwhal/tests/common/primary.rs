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

use snarkos_account::Account;
use snarkos_node_narwhal::{
    helpers::{init_primary_channels, Committee, PrimarySender, Storage},
    Primary,
    MAX_GC_ROUNDS,
    MEMORY_POOL_PORT,
};

use tracing::*;

use anyhow::Result;

use indexmap::IndexMap;
use parking_lot::RwLock;
use rand::SeedableRng;
use std::{net::SocketAddr, str::FromStr, sync::Arc};

use crate::common::CurrentNetwork;

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
