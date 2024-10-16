// Copyright 2024 Aleo Network Foundation
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

// Avoid a false positive from clippy:
// https://github.com/rust-lang/rust-clippy/issues/6446
#![allow(clippy::await_holding_lock)]

use snarkvm::prelude::{
    block::Block,
    store::{cow_to_copied, ConsensusStorage},
    Deserialize,
    DeserializeOwned,
    Ledger,
    Network,
    Serialize,
};

use anyhow::{anyhow, bail, Result};
use colored::Colorize;
use parking_lot::Mutex;
use reqwest::Client;
use std::{
    cmp,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

/// The number of blocks per file.
const BLOCKS_PER_FILE: u32 = 50;
/// The desired number of concurrent requests to the CDN.
const CONCURRENT_REQUESTS: u32 = 16;
/// Maximum number of pending sync blocks.
const MAXIMUM_PENDING_BLOCKS: u32 = BLOCKS_PER_FILE * CONCURRENT_REQUESTS * 2;
/// Maximum number of attempts for a request to the CDN.
const MAXIMUM_REQUEST_ATTEMPTS: u8 = 10;

/// Loads blocks from a CDN into the ledger.
///
/// On success, this function returns the completed block height.
/// On failure, this function returns the last successful block height (if any), along with the error.
pub async fn sync_ledger_with_cdn<N: Network, C: ConsensusStorage<N>>(
    base_url: &str,
    ledger: Ledger<N, C>,
    shutdown: Arc<AtomicBool>,
) -> Result<u32, (u32, anyhow::Error)> {
    // Fetch the node height.
    let start_height = ledger.latest_height() + 1;
    // Load the blocks from the CDN into the ledger.
    let ledger_clone = ledger.clone();
    let result = load_blocks(base_url, start_height, None, shutdown, move |block: Block<N>| {
        ledger_clone.advance_to_next_block(&block)
    })
    .await;

    // TODO (howardwu): Find a way to resolve integrity failures.
    // If the sync failed, check the integrity of the ledger.
    if let Err((completed_height, error)) = &result {
        warn!("{error}");

        // If the sync made any progress, then check the integrity of the ledger.
        if *completed_height != start_height {
            debug!("Synced the ledger up to block {completed_height}");

            // Retrieve the latest height, according to the ledger.
            let node_height = cow_to_copied!(ledger.vm().block_store().heights().max().unwrap_or_default());
            // Check the integrity of the latest height.
            if &node_height != completed_height {
                return Err((*completed_height, anyhow!("The ledger height does not match the last sync height")));
            }

            // Fetch the latest block from the ledger.
            if let Err(err) = ledger.get_block(node_height) {
                return Err((*completed_height, err));
            }
        }

        Ok(*completed_height)
    } else {
        result
    }
}

/// Loads blocks from a CDN and process them with the given function.
///
/// On success, this function returns the completed block height.
/// On failure, this function returns the last successful block height (if any), along with the error.
pub async fn load_blocks<N: Network>(
    base_url: &str,
    start_height: u32,
    end_height: Option<u32>,
    shutdown: Arc<AtomicBool>,
    process: impl FnMut(Block<N>) -> Result<()> + Clone + Send + Sync + 'static,
) -> Result<u32, (u32, anyhow::Error)> {
    // Create a Client to maintain a connection pool throughout the sync.
    let client = match Client::builder().build() {
        Ok(client) => client,
        Err(error) => {
            return Err((start_height.saturating_sub(1), anyhow!("Failed to create a CDN request client - {error}")));
        }
    };

    // Fetch the CDN height.
    let cdn_height = match cdn_height::<BLOCKS_PER_FILE>(&client, base_url).await {
        Ok(cdn_height) => cdn_height,
        Err(error) => return Err((start_height, error)),
    };
    // If the CDN height is less than the start height, return.
    if cdn_height < start_height {
        return Err((
            start_height,
            anyhow!("The given start height ({start_height}) must be less than the CDN height ({cdn_height})"),
        ));
    }

    // If the end height is not specified, set it to the CDN height.
    // If the end height is greater than the CDN height, set the end height to the CDN height.
    let end_height = cmp::min(end_height.unwrap_or(cdn_height), cdn_height);
    // If the end height is less than the start height, return.
    if end_height < start_height {
        return Err((
            start_height,
            anyhow!("The given end height ({end_height}) must not be less than the start height ({start_height})"),
        ));
    }

    // Compute the CDN start height rounded down to the nearest multiple.
    let cdn_start = start_height - (start_height % BLOCKS_PER_FILE);
    // Set the CDN end height to the given end height.
    let cdn_end = end_height;
    // If the CDN range is empty, return.
    if cdn_start >= cdn_end {
        return Ok(cdn_end);
    }

    // A collection of downloaded blocks pending insertion into the ledger.
    let pending_blocks: Arc<Mutex<Vec<Block<N>>>> = Default::default();

    // Start a timer.
    let timer = Instant::now();

    // Spawn a background task responsible for concurrent downloads.
    let pending_blocks_clone = pending_blocks.clone();
    let base_url = base_url.to_owned();
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        download_block_bundles(client, base_url, cdn_start, cdn_end, pending_blocks_clone, shutdown_clone).await;
    });

    // A loop for inserting the pending blocks into the ledger.
    let mut current_height = start_height.saturating_sub(1);
    while current_height < end_height - 1 {
        // If we are instructed to shut down, abort.
        if shutdown.load(Ordering::Relaxed) {
            info!("Stopping block sync at {} - shutting down", current_height);
            // We can shut down cleanly from here, as the node hasn't been started yet.
            std::process::exit(0);
        }

        let mut candidate_blocks = pending_blocks.lock();

        // Obtain the height of the nearest pending block.
        let Some(next_height) = candidate_blocks.first().map(|b| b.height()) else {
            debug!("No pending blocks yet");
            drop(candidate_blocks);
            tokio::time::sleep(Duration::from_secs(3)).await;
            continue;
        };

        // Wait if the nearest pending block is not the next one that can be inserted.
        if next_height > current_height + 1 {
            // There is a gap in pending blocks, we need to wait.
            debug!("Waiting for the first relevant blocks ({} pending)", candidate_blocks.len());
            drop(candidate_blocks);
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        // Obtain the first BLOCKS_PER_FILE applicable blocks.
        let retained_blocks = candidate_blocks.split_off(BLOCKS_PER_FILE as usize);
        let next_blocks = std::mem::replace(&mut *candidate_blocks, retained_blocks);
        drop(candidate_blocks);

        // Attempt to advance the ledger using the CDN block bundle.
        let mut process_clone = process.clone();
        let shutdown_clone = shutdown.clone();
        current_height = tokio::task::spawn_blocking(move || {
            for block in next_blocks.into_iter().filter(|b| (start_height..end_height).contains(&b.height())) {
                // If we are instructed to shut down, abort.
                if shutdown_clone.load(Ordering::Relaxed) {
                    info!("Stopping block sync at {} - the node is shutting down", current_height);
                    // We can shut down cleanly from here, as the node hasn't been started yet.
                    std::process::exit(0);
                }

                // Register the next block's height, as the block gets consumed next.
                let block_height = block.height();

                // Insert the block into the ledger.
                process_clone(block)?;

                // Update the current height.
                current_height = block_height;

                // Log the progress.
                log_progress::<BLOCKS_PER_FILE>(timer, current_height, cdn_start, cdn_end, "block");
            }

            Ok(current_height)
        })
        .await
        .map_err(|e| (current_height, e.into()))?
        .map_err(|e| (current_height, e))?;
    }

    Ok(current_height)
}

async fn download_block_bundles<N: Network>(
    client: Client,
    base_url: String,
    cdn_start: u32,
    cdn_end: u32,
    pending_blocks: Arc<Mutex<Vec<Block<N>>>>,
    shutdown: Arc<AtomicBool>,
) {
    // Keep track of the number of concurrent requests.
    let active_requests: Arc<AtomicU32> = Default::default();

    let mut start = cdn_start;
    while start < cdn_end - 1 {
        // If we are instructed to shut down, stop downloading.
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        // Avoid collecting too many blocks in order to restrict memory use.
        let num_pending_blocks = pending_blocks.lock().len();
        if num_pending_blocks >= MAXIMUM_PENDING_BLOCKS as usize {
            debug!("Maximum number of pending blocks reached ({num_pending_blocks}), waiting...");
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        // The number of concurrent requests is maintained at CONCURRENT_REQUESTS, unless the maximum
        // number of pending blocks may be breached.
        let active_request_count = active_requests.load(Ordering::Relaxed);
        let num_requests =
            cmp::min(CONCURRENT_REQUESTS, (MAXIMUM_PENDING_BLOCKS - num_pending_blocks as u32) / BLOCKS_PER_FILE)
                .saturating_sub(active_request_count);

        // Spawn concurrent requests for bundles of blocks.
        for i in 0..num_requests {
            let start = start + i * BLOCKS_PER_FILE;
            let end = start + BLOCKS_PER_FILE;

            // If this request would breach the upper limit, stop downloading.
            if end > cdn_end + BLOCKS_PER_FILE {
                debug!("Finishing network requests to the CDN...");
                break;
            }

            let client_clone = client.clone();
            let base_url_clone = base_url.clone();
            let pending_blocks_clone = pending_blocks.clone();
            let active_requests_clone = active_requests.clone();
            let shutdown_clone = shutdown.clone();
            tokio::spawn(async move {
                // Increment the number of active requests.
                active_requests_clone.fetch_add(1, Ordering::Relaxed);

                let ctx = format!("blocks {start} to {end}");
                debug!("Requesting {ctx} (of {cdn_end})");

                // Prepare the URL.
                let blocks_url = format!("{base_url_clone}/{start}.{end}.blocks");
                let ctx = format!("blocks {start} to {end}");
                // Download blocks, retrying on failure.
                let mut attempts = 0;
                let request_time = Instant::now();

                loop {
                    // Fetch the blocks.
                    match cdn_get(client_clone.clone(), &blocks_url, &ctx).await {
                        Ok::<Vec<Block<N>>, _>(blocks) => {
                            // Keep the collection of pending blocks sorted by the height.
                            let mut pending_blocks = pending_blocks_clone.lock();
                            for block in blocks {
                                match pending_blocks.binary_search_by_key(&block.height(), |b| b.height()) {
                                    Ok(_idx) => warn!("Found a duplicate pending block at height {}", block.height()),
                                    Err(idx) => pending_blocks.insert(idx, block),
                                }
                            }
                            debug!("Received {ctx} {}", format!("(in {:.2?})", request_time.elapsed()).dimmed());
                            break;
                        }
                        Err(error) => {
                            // Increment the attempt counter, and wait with a linear backoff, or abort in
                            // case the maximum number of attempts has been breached.
                            attempts += 1;
                            if attempts > MAXIMUM_REQUEST_ATTEMPTS {
                                warn!("Maximum number of requests to {blocks_url} reached - shutting down...");
                                shutdown_clone.store(true, Ordering::Relaxed);
                                break;
                            }
                            tokio::time::sleep(Duration::from_secs(attempts as u64 * 10)).await;
                            warn!("{error} - retrying ({attempts} attempt(s) so far)");
                        }
                    }
                }

                // Decrement the number of active requests.
                active_requests_clone.fetch_sub(1, Ordering::Relaxed);
            });
        }

        // Increase the starting block height for the subsequent requests.
        start += BLOCKS_PER_FILE * num_requests;

        // A short sleep in order to allow some block processing to happen in the meantime.
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    debug!("Finished network requests to the CDN");
}

/// Retrieves the CDN height with the given base URL.
///
/// Note: This function decrements the tip by a few blocks, to ensure the
/// tip is not on a block that is not yet available on the CDN.
async fn cdn_height<const BLOCKS_PER_FILE: u32>(client: &Client, base_url: &str) -> Result<u32> {
    // A representation of the 'latest.json' file object.
    #[derive(Deserialize, Serialize, Debug)]
    struct LatestState {
        exclusive_height: u32,
        inclusive_height: u32,
        hash: String,
    }
    // Prepare the URL.
    let latest_json_url = format!("{base_url}/latest.json");
    // Send the request.
    let response = match client.get(latest_json_url).send().await {
        Ok(response) => response,
        Err(error) => bail!("Failed to fetch the CDN height - {error}"),
    };
    // Parse the response.
    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => bail!("Failed to parse the CDN height response - {error}"),
    };
    // Parse the bytes for the string.
    let latest_state_string = match bincode::deserialize::<String>(&bytes) {
        Ok(string) => string,
        Err(error) => bail!("Failed to deserialize the CDN height response - {error}"),
    };
    // Parse the string for the tip.
    let tip = match serde_json::from_str::<LatestState>(&latest_state_string) {
        Ok(latest) => latest.exclusive_height,
        Err(error) => bail!("Failed to extract the CDN height response - {error}"),
    };
    // Decrement the tip by a few blocks to ensure the CDN is caught up.
    let tip = tip.saturating_sub(10);
    // Adjust the tip to the closest subsequent multiple of BLOCKS_PER_FILE.
    Ok(tip - (tip % BLOCKS_PER_FILE) + BLOCKS_PER_FILE)
}

/// Retrieves the objects from the CDN with the given URL.
async fn cdn_get<T: 'static + DeserializeOwned + Send>(client: Client, url: &str, ctx: &str) -> Result<T> {
    // Fetch the bytes from the given URL.
    let response = match client.get(url).send().await {
        Ok(response) => response,
        Err(error) => bail!("Failed to fetch {ctx} - {error}"),
    };
    // Parse the response.
    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => bail!("Failed to parse {ctx} - {error}"),
    };
    // Parse the objects.
    match tokio::task::spawn_blocking(move || bincode::deserialize::<T>(&bytes)).await {
        Ok(Ok(objects)) => Ok(objects),
        Ok(Err(error)) => bail!("Failed to deserialize {ctx} - {error}"),
        Err(error) => bail!("Failed to join task for {ctx} - {error}"),
    }
}

/// Logs the progress of the sync.
fn log_progress<const OBJECTS_PER_FILE: u32>(
    timer: Instant,
    current_index: u32,
    cdn_start: u32,
    mut cdn_end: u32,
    object_name: &str,
) {
    // Subtract 1, as the end of the range is exclusive.
    cdn_end -= 1;
    // Compute the percentage completed.
    let percentage = current_index * 100 / cdn_end;
    // Compute the number of files processed so far.
    let num_files_done = 1 + (current_index - cdn_start) / OBJECTS_PER_FILE;
    // Compute the number of files remaining.
    let num_files_remaining = 1 + (cdn_end.saturating_sub(current_index)) / OBJECTS_PER_FILE;
    // Compute the milliseconds per file.
    let millis_per_file = timer.elapsed().as_millis() / num_files_done as u128;
    // Compute the heuristic slowdown factor (in millis).
    let slowdown = 100 * num_files_remaining as u128;
    // Compute the time remaining (in millis).
    let time_remaining = num_files_remaining as u128 * millis_per_file + slowdown;
    // Prepare the estimate message (in secs).
    let estimate = format!("(est. {} minutes remaining)", time_remaining / (60 * 1000));
    // Log the progress.
    info!("Synced up to {object_name} {current_index} of {cdn_end} - {percentage}% complete {}", estimate.dimmed());
}

#[cfg(test)]
mod tests {
    use crate::{
        blocks::{cdn_get, cdn_height, log_progress, BLOCKS_PER_FILE},
        load_blocks,
    };
    use snarkvm::prelude::{block::Block, MainnetV0};

    use parking_lot::RwLock;
    use std::{sync::Arc, time::Instant};

    type CurrentNetwork = MainnetV0;

    const TEST_BASE_URL: &str = "https://s3.us-west-1.amazonaws.com/testnet3.blocks/phase3";

    fn check_load_blocks(start: u32, end: Option<u32>, expected: usize) {
        let blocks = Arc::new(RwLock::new(Vec::new()));
        let blocks_clone = blocks.clone();
        let process = move |block: Block<CurrentNetwork>| {
            blocks_clone.write().push(block);
            Ok(())
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let completed_height = load_blocks(TEST_BASE_URL, start, end, Default::default(), process).await.unwrap();
            assert_eq!(blocks.read().len(), expected);
            if expected > 0 {
                assert_eq!(blocks.read().last().unwrap().height(), completed_height);
            }
            // Check they are sequential.
            for (i, block) in blocks.read().iter().enumerate() {
                assert_eq!(block.height(), start + i as u32);
            }
        });
    }

    #[test]
    fn test_load_blocks_0_to_50() {
        let start_height = 0;
        let end_height = Some(50);
        check_load_blocks(start_height, end_height, 50);
    }

    #[test]
    fn test_load_blocks_50_to_100() {
        let start_height = 50;
        let end_height = Some(100);
        check_load_blocks(start_height, end_height, 50);
    }

    #[test]
    fn test_load_blocks_0_to_123() {
        let start_height = 0;
        let end_height = Some(123);
        check_load_blocks(start_height, end_height, 123);
    }

    #[test]
    fn test_load_blocks_46_to_234() {
        let start_height = 46;
        let end_height = Some(234);
        check_load_blocks(start_height, end_height, 188);
    }

    #[test]
    fn test_cdn_height() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::builder().build().unwrap();
        rt.block_on(async {
            let height = cdn_height::<BLOCKS_PER_FILE>(&client, TEST_BASE_URL).await.unwrap();
            assert!(height > 0);
        });
    }

    #[test]
    fn test_cdn_get() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = reqwest::Client::new();
            let height =
                cdn_get::<u32>(client, &format!("{TEST_BASE_URL}/mainnet/latest/height"), "height").await.unwrap();
            assert!(height > 0);
        });
    }

    #[test]
    fn test_log_progress() {
        // This test sanity checks that basic arithmetic is correct (i.e. no divide by zero, etc.).
        let timer = Instant::now();
        let cdn_start = 0;
        let cdn_end = 100;
        let object_name = "blocks";
        log_progress::<10>(timer, 0, cdn_start, cdn_end, object_name);
        log_progress::<10>(timer, 10, cdn_start, cdn_end, object_name);
        log_progress::<10>(timer, 20, cdn_start, cdn_end, object_name);
        log_progress::<10>(timer, 30, cdn_start, cdn_end, object_name);
        log_progress::<10>(timer, 40, cdn_start, cdn_end, object_name);
        log_progress::<10>(timer, 50, cdn_start, cdn_end, object_name);
        log_progress::<10>(timer, 60, cdn_start, cdn_end, object_name);
        log_progress::<10>(timer, 70, cdn_start, cdn_end, object_name);
        log_progress::<10>(timer, 80, cdn_start, cdn_end, object_name);
        log_progress::<10>(timer, 90, cdn_start, cdn_end, object_name);
        log_progress::<10>(timer, 100, cdn_start, cdn_end, object_name);
    }
}
