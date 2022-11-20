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

use snarkos_node_ledger::Ledger;
use snarkvm::prelude::{Block, ConsensusStorage, Network};

use anyhow::{anyhow, bail};
use backoff::{future::retry, ExponentialBackoff};
use colored::Colorize;
use futures::{Future, StreamExt};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

/// Syncs the ledger with the network.
pub async fn load_blocks<N: Network, C: ConsensusStorage<N>>(cdn: Option<String>, ledger: Ledger<N, C>) {
    /// The number of blocks per file.
    const BLOCKS_PER_FILE: u32 = 50;
    /// TODO (howardwu): Change this with Phase 3.
    /// The current phase.
    const PHASE: &str = "phase2";

    // If the network is not Aleo Testnet 3, return (other networks are not supported yet).
    if N::ID != 3 {
        return;
    }

    // If the CDN is not specified, return.
    let url = match cdn {
        Some(url) => url,
        None => return,
    };

    // Create a Client to maintain a connection pool throughout the sync.
    let client = match reqwest::Client::builder().build() {
        Ok(client) => client,
        Err(error) => {
            warn!("Failed to create a CDN request client: {error}");
            return;
        }
    };

    // Fetch the CDN height.
    let cdn_height = {
        // Prepare the URL.
        let height_url = format!("{url}/testnet3/latest/height");
        // Send the request.
        let response = match client.get(height_url).send().await {
            Ok(response) => response,
            Err(error) => {
                warn!("Failed to fetch the CDN height: {error}");
                return;
            }
        };
        // Parse the response.
        let text = match response.text().await {
            Ok(text) => text,
            Err(error) => {
                warn!("Failed to parse the CDN height response: {error}");
                return;
            }
        };
        // Parse the tip.
        let tip = match text.parse::<u32>() {
            Ok(tip) => tip,
            Err(error) => {
                warn!("Failed to parse the CDN tip: {error}");
                return;
            }
        };
        // Decrement the tip by a few blocks to ensure the CDN is caught up.
        let tip = tip.saturating_sub(10);
        // Round the tip down to the nearest multiple.
        tip - (tip % BLOCKS_PER_FILE)
    };

    // Start a timer.
    let timer = std::time::Instant::now();

    // Fetch the node height.
    let node_height = ledger.latest_height();

    // Sync the node to the CDN height.
    if cdn_height > node_height + 1 {
        // Compute the start height rounded down to the nearest multiple.
        let start_height = node_height - (node_height % BLOCKS_PER_FILE);
        // Set the end height to the CDN height.
        let end_height = cdn_height;

        // An atomic boolean to indicate if the sync failed.
        // This is a hack to ensure the future does not panic.
        let failed = Arc::new(AtomicBool::new(false));

        futures::stream::iter((start_height..end_height).step_by(BLOCKS_PER_FILE as usize))
            .map(|start| {
                // Prepare the end height.
                let end = start + BLOCKS_PER_FILE;
                debug!("Requesting blocks {start} to {end} (of {cdn_height})");

                // Download the blocks with an exponential backoff retry policy.
                let client_clone = client.clone();
                let url_clone = url.clone();
                let failed_clone = failed.clone();
                handle_dispatch_error(move || {
                    let client = client_clone.clone();
                    let url = url_clone.clone();
                    let failed = failed_clone.clone();
                    async move {
                        // If the sync failed, return with an empty vector.
                        if failed.load(Ordering::SeqCst) {
                            return std::future::ready(Ok(vec![])).await
                        }

                        // Prepare the URL.
                        let blocks_url = format!("{url}/testnet3/blocks/{PHASE}/{start}.{end}.blocks");
                        // Fetch the bytes from the given URL.
                        let response = match client.get(blocks_url).send().await {
                            Ok(response) => response,
                            Err(error) => bail!("Failed to fetch blocks {start} to {end}: {error}")
                        };
                        // Parse the response.
                        let blocks_bytes = match response.bytes().await {
                            Ok(blocks_bytes) => blocks_bytes,
                            Err(error) => bail!("Failed to parse blocks {start} to {end}: {error}")
                        };
                        // Parse the blocks.
                        let blocks = match tokio::task::spawn_blocking(move || bincode::deserialize::<Vec<Block<N>>>(&blocks_bytes)).await {
                            Ok(Ok(blocks)) => blocks,
                            Ok(Err(error)) => bail!("Failed to deserialize {start} to {end}: {error}"),
                            Err(error) => bail!("Failed to join task for {start} to {end}: {error}")
                        };
                        std::future::ready(Ok(blocks)).await
                    }
                })
            })
            .buffered(512) // The number of concurrent requests.
            .for_each(|result| async {
                // If the sync previously failed, return early.
                if failed.load(Ordering::SeqCst) {
                    return;
                }

                // Unwrap the blocks.
                let mut blocks = match result {
                    Ok(blocks) => blocks,
                    Err(error) => {
                        warn!("{error}");
                        failed.store(true, Ordering::SeqCst);
                        return;
                    }
                };

                // Only retain blocks that are in the ledger.
                blocks.retain(|block| block.height() > node_height);

                #[cfg(debug_assertions)]
                // Ensure the blocks are in order.
                for (i, block) in blocks.iter().enumerate() {
                    if i > 0 {
                        assert_eq!(block.height(), blocks[i - 1].height() + 1);
                    }
                }

                // Use blocking tasks, as deserialization and adding blocks are expensive operations.
                let ledger_clone = ledger.clone();
                let failed_clone = failed.clone();
                let result = tokio::task::spawn_blocking(move || {
                    // Fetch the last height in the blocks.
                    let curr_height = blocks.last().map(|block| block.height()).unwrap_or(node_height);

                    // Add the blocks to the ledger.
                    for block in blocks {
                        // If the sync failed, set the failed flag, and return.
                        if let Err(error) = ledger_clone.add_next_block(&block) {
                            warn!("Failed to add block {} to the ledger: {error}", block.height());
                            failed_clone.store(true, Ordering::SeqCst);
                            return;
                        }
                    }

                    // Compute the percentage completed.
                    let percentage = curr_height * 100 / cdn_height;
                    // Compute the number of files processed so far.
                    let num_files_done = 1 + (curr_height - start_height) / BLOCKS_PER_FILE;
                    // Compute the number of files remaining.
                    let num_files_remaining = 1 + (cdn_height - curr_height) / BLOCKS_PER_FILE;
                    // Compute the milliseconds per file.
                    let millis_per_file = timer.elapsed().as_millis() / num_files_done as u128;
                    // Compute the heuristic slowdown factor (in millis).
                    let slowdown = 100 * num_files_remaining as u128;
                    // Compute the time remaining (in millis).
                    let time_remaining = num_files_remaining as u128 * millis_per_file + slowdown;
                    // Prepare the estimate message (in secs).
                    let estimate = format!("(est. {} minutes remaining)", time_remaining / (60 * 1000));
                    // Log the progress.
                    info!(
                        "Synced up to block {curr_height} of {cdn_height} - {percentage}% complete {}",
                        estimate.dimmed()
                    );
                }).await;

                // If the sync failed, set the failed flag.
                if result.is_err() {
                    failed.store(true, Ordering::SeqCst);
                }
            })
            .await;
    }
}

pub(crate) async fn handle_dispatch_error<'a, T, F>(func: impl Fn() -> F + 'a) -> anyhow::Result<T>
where
    F: Future<Output = Result<T, anyhow::Error>>,
{
    fn default_backoff() -> ExponentialBackoff {
        ExponentialBackoff {
            max_interval: Duration::from_secs(15),
            max_elapsed_time: Some(Duration::from_secs(60)),
            ..Default::default()
        }
    }

    fn from_anyhow_err(err: anyhow::Error) -> backoff::Error<anyhow::Error> {
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
