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

use indexmap::IndexMap;
use parking_lot::RwLock;
use rand::SeedableRng;
use std::{collections::HashMap, net::SocketAddr, str::FromStr, sync::Arc};

use crate::common::CurrentNetwork;

// Initializes a new test committee.
pub fn new_test_committee(n: u16) -> (Vec<Account<CurrentNetwork>>, Committee<CurrentNetwork>) {
    const INITIAL_STAKE: u64 = 1000;

    let mut accounts = Vec::with_capacity(n as usize);
    let mut members = IndexMap::with_capacity(n as usize);
    for i in 0..n {
        // Sample the account.
        let account = Account::new(&mut rand_chacha::ChaChaRng::seed_from_u64(i as u64)).unwrap();
        members.insert(account.address(), INITIAL_STAKE);
        accounts.push(account);
        // TODO(nkls): use tracing instead.
        // println!("  Validator {}: {}", i, account.address());
    }
    // Initialize the committee.
    let committee = Committee::<CurrentNetwork>::new(1u64, members).unwrap();

    (accounts, committee)
}

pub async fn start_n_primaries(n: u16) -> HashMap<u16, (Primary<CurrentNetwork>, PrimarySender<CurrentNetwork>)> {
    let mut primaries = HashMap::with_capacity(n as usize);
    let (accounts, committee) = new_test_committee(n);

    for (n, account) in accounts.into_iter().enumerate() {
        let storage = Storage::new(MAX_GC_ROUNDS);
        let (sender, receiver) = init_primary_channels();
        let mut primary =
            Primary::<CurrentNetwork>::new(Arc::new(RwLock::new(committee.clone())), storage, account, Some(n as u16))
                .unwrap();

        primary.run(sender.clone(), receiver).await.unwrap();
        primaries.insert(n as u16, (primary, sender));
    }

    primaries
}

// TODO(nkls): should be handled by the gateway or on the snarkOS level.
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
