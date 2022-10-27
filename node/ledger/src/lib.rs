// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

#![forbid(unsafe_code)]

#[macro_use]
extern crate tracing;

pub mod consensus;
pub use consensus::*;

mod memory_pool;

use snarkvm::prelude::*;

use anyhow::{anyhow, ensure, Result};
use colored::Colorize;
use core::time::Duration;
use futures::{Future, StreamExt};
use indexmap::IndexMap;
use parking_lot::RwLock;
use std::{net::IpAddr, sync::Arc};
use tokio::task;

type RecordMap<N> = IndexMap<Field<N>, Record<N, Plaintext<N>>>;

#[derive(Clone)]
pub struct Ledger<N: Network, C: ConsensusStorage<N>> {
    /// The consensus module.
    consensus: Arc<RwLock<Consensus<N, C>>>,
    /// The account private key.
    private_key: PrivateKey<N>,
    /// The account view key.
    view_key: ViewKey<N>,
    /// The account address.
    address: Address<N>,
}

impl<N: Network, C: ConsensusStorage<N>> Ledger<N, C> {
    /// Loads an instance of the ledger.
    pub fn load(private_key: PrivateKey<N>, genesis: Option<Block<N>>, dev: Option<u16>) -> Result<Self> {
        // Initialize consensus.
        let consensus = Arc::new(RwLock::new(Consensus::load(genesis, dev)?));
        // Return the ledger.
        Self::from(consensus, private_key)
    }

    /// Initializes a new instance of the ledger.
    pub fn from(consensus: Arc<RwLock<Consensus<N, C>>>, private_key: PrivateKey<N>) -> Result<Self> {
        // Derive the view key and address.
        let view_key = ViewKey::try_from(private_key)?;
        let address = Address::try_from(&view_key)?;

        // Return the ledger.
        Ok(Self { consensus, private_key, view_key, address })
    }

    /// Returns the consensus module.
    pub const fn consensus(&self) -> &Arc<RwLock<Consensus<N, C>>> {
        &self.consensus
    }

    /// Returns the ledger address.
    pub const fn address(&self) -> Address<N> {
        self.address
    }

    /// Adds the given unconfirmed transaction to the memory pool.
    pub fn add_unconfirmed_transaction(&self, transaction: Transaction<N>) -> Result<()> {
        self.consensus.write().add_unconfirmed_transaction(transaction)
    }

    /// Adds the given unconfirmed solution to the memory pool.
    pub fn add_unconfirmed_solution(&self, solution: &ProverSolution<N>) -> Result<()> {
        self.consensus.write().add_unconfirmed_solution(solution)
    }

    /// Returns the unspent records.
    pub fn find_unspent_records(&self) -> Result<RecordMap<N>> {
        Ok(self
            .consensus
            .read()
            .find_records(&self.view_key, RecordsFilter::Unspent)?
            .filter(|(_, record)| !record.gates().is_zero())
            .collect::<IndexMap<_, _>>())
    }

    /// Returns the spent records.
    pub fn find_spent_records(&self) -> Result<RecordMap<N>> {
        Ok(self
            .consensus
            .read()
            .find_records(&self.view_key, RecordsFilter::Spent)?
            .filter(|(_, record)| !record.gates().is_zero())
            .collect::<IndexMap<_, _>>())
    }

    /// Creates a deploy transaction.
    pub fn create_deploy(&self, program: &Program<N>, additional_fee: u64) -> Result<Transaction<N>> {
        // Fetch the unspent records.
        let records = self.find_unspent_records()?;
        ensure!(!records.len().is_zero(), "The Aleo account has no records to spend.");

        // Prepare the additional fee.
        let credits = records.values().max_by(|a, b| (**a.gates()).cmp(&**b.gates())).unwrap().clone();
        ensure!(***credits.gates() >= additional_fee, "The additional fee is more than the record balance.");

        // Initialize an RNG.
        let rng = &mut ::rand::thread_rng();
        // Deploy.
        let transaction = Transaction::deploy(
            self.consensus.read().vm(),
            &self.private_key,
            program,
            (credits, additional_fee),
            rng,
        )?;
        // Verify.
        assert!(self.consensus.read().vm().verify(&transaction));
        // Return the transaction.
        Ok(transaction)
    }

    /// Creates a transfer transaction.
    pub fn create_transfer(&self, to: &Address<N>, amount: u64) -> Result<Transaction<N>> {
        // Fetch the unspent records.
        let records = self.find_unspent_records()?;
        ensure!(!records.len().is_zero(), "The Aleo account has no records to spend.");

        // Initialize an RNG.
        let rng = &mut ::rand::thread_rng();

        // Create a new transaction.
        Transaction::execute(
            self.consensus.read().vm(),
            &self.private_key,
            &ProgramID::from_str("credits.aleo")?,
            Identifier::from_str("transfer")?,
            &[
                Value::Record(records.values().next().unwrap().clone()),
                Value::from_str(&format!("{to}"))?,
                Value::from_str(&format!("{amount}u64"))?,
            ],
            None,
            rng,
        )
    }

    /// Syncs the ledger with the network.
    #[allow(dead_code)]
    pub(crate) async fn initial_sync_with_network(self: &Arc<Self>, leader_ip: IpAddr) -> Result<()> {
        /// The number of concurrent requests with the network.
        const CONCURRENT_REQUESTS: usize = 100;
        /// Url to fetch the blocks from.
        const TARGET_URL: &str = "https://vm.aleo.org/testnet3/block/testnet3/";

        async fn handle_dispatch_error<'a, T, F>(func: impl Fn() -> F + 'a) -> Result<T>
        where
            F: Future<Output = Result<T, Error>>,
        {
            use backoff::{future::retry, ExponentialBackoff};

            fn default_backoff() -> ExponentialBackoff {
                ExponentialBackoff {
                    max_interval: Duration::from_secs(10),
                    max_elapsed_time: Some(Duration::from_secs(45)),
                    ..Default::default()
                }
            }

            fn from_anyhow_err(err: Error) -> backoff::Error<Error> {
                use backoff::Error;

                if let Ok(err) = err.downcast::<reqwest::Error>() {
                    debug!("Server error: {err}; retrying...");
                    Error::Transient { err: err.into(), retry_after: None }
                } else {
                    Error::Transient { err: anyhow!("Block parse error"), retry_after: None }
                }
            }

            retry(default_backoff(), || async { func().await.map_err(from_anyhow_err) }).await
        }

        // Fetch the ledger height.
        let ledger_height = self.consensus.read().latest_height();

        // Create a Client to maintain a connection pool throughout the sync.
        let client = reqwest::Client::builder().build()?;

        // Fetch the latest height.
        let latest_height = client
            .get(format!("http://{leader_ip}/testnet3/latest/height"))
            .send()
            .await?
            .text()
            .await?
            .parse::<u32>()?;

        // Start a timer.
        let timer = std::time::Instant::now();

        // Sync the ledger to the latest block height.
        if latest_height > ledger_height + 1 {
            futures::stream::iter((ledger_height + 1)..=latest_height)
                .map(|height| {
                    trace!("Requesting block {height} of {latest_height}");

                    // Download the block with an exponential backoff retry policy.
                    let client_clone = client.clone();
                    handle_dispatch_error(move || {
                        let client = client_clone.clone();
                        async move {
                            // Get the URL for the block download.
                            let block_url = format!("{TARGET_URL}{height}.block");

                            // Fetch the bytes from the given url
                            let block_bytes = client.get(block_url).send().await?.bytes().await?;

                            // Parse the block.
                            let block =
                                task::spawn_blocking(move || Block::from_bytes_le(&block_bytes)).await.unwrap()?;

                            std::future::ready(Ok(block)).await
                        }
                    })
                })
                .buffered(CONCURRENT_REQUESTS)
                .for_each(|block| async {
                    let block = block.unwrap();
                    // Use blocking tasks, as deserialization and adding blocks are expensive operations.
                    let self_clone = self.clone();

                    task::spawn_blocking(move || {
                        // Add the block to the ledger.
                        self_clone.consensus.write().add_next_block(&block).unwrap();

                        // Retrieve the current height.
                        let height = block.height();
                        // Compute the percentage completed.
                        let percentage = height * 100 / latest_height;
                        // Compute the heuristic slowdown factor (in millis).
                        let slowdown = (100 * (latest_height - height)) as u128;
                        // Compute the time remaining (in millis).
                        let millis_per_block = (timer.elapsed().as_millis()) / (height - ledger_height) as u128;
                        let time_remaining = (latest_height - height) as u128 * millis_per_block + slowdown;
                        // Prepare the estimate message (in secs).
                        let estimate = format!("(est. {} minutes remaining)", time_remaining / (60 * 1000));
                        // Log the progress.
                        info!(
                            "Synced up to block {height} of {latest_height} - {percentage}% complete {}",
                            estimate.dimmed()
                        );
                    })
                    .await
                    .unwrap();
                })
                .await;
        }

        Ok(())
    }
}
