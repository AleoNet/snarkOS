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
use snarkvm::prelude::{cow_to_copied, Block, ConsensusStorage, DeserializeOwned, Network};

use anyhow::{anyhow, bail, Result};
use colored::Colorize;
use core::ops::Range;
use futures::{Future, StreamExt};
use parking_lot::RwLock;
use reqwest::Client;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// The number of blocks per file.
const BLOCKS_PER_FILE: u32 = 50;
/// The supported network.
const NETWORK_ID: u16 = 3;

/// Loads blocks from a CDN into the ledger.
///
/// On success, this function returns the completed block height.
/// On failure, this function returns the last successful block height (if any), along with the error.
pub async fn sync_ledger_with_cdn<N: Network, C: ConsensusStorage<N>>(
    base_url: &str,
    ledger: Ledger<N, C>,
) -> Result<u32, (u32, anyhow::Error)> {
    // Fetch the node height.
    let start_height = ledger.latest_height() + 1;
    // Load the blocks from the CDN into the ledger.
    let ledger_clone = ledger.clone();
    let result =
        load_blocks(base_url, start_height, None, move |block: Block<N>| ledger_clone.add_next_block(&block)).await;

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
    process: impl FnMut(Block<N>) -> Result<()> + Clone + Send + Sync + 'static,
) -> Result<u32, (u32, anyhow::Error)> {
    // If the network is not supported, return.
    if N::ID != NETWORK_ID {
        return Err((start_height, anyhow!("The network ({}) is not supported", N::ID)));
    }

    // Fetch the CDN height.
    let cdn_height = match cdn_height::<BLOCKS_PER_FILE>().await {
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
    let end_height = end_height.unwrap_or(cdn_height);
    // If the end height is greater than the CDN height, set the end height to the CDN height.
    let end_height = if end_height > cdn_height { cdn_height } else { end_height };
    // If the end height is less than the start height, return.
    if end_height < start_height {
        return Err((
            start_height,
            anyhow!("The given end height ({end_height}) must be less than the start height ({start_height})"),
        ));
    }

    // Compute the CDN start height rounded down to the nearest multiple.
    let cdn_start = start_height - (start_height % BLOCKS_PER_FILE);
    // Set the CDN end height to the given end height.
    let cdn_end = end_height;
    // Construct the CDN range.
    let cdn_range = cdn_start..cdn_end;
    // If the CDN range is empty, return.
    if cdn_range.is_empty() {
        return Ok(cdn_end);
    }

    // Create a Client to maintain a connection pool throughout the sync.
    let client = match Client::builder().build() {
        Ok(client) => client,
        Err(error) => return Err((start_height, anyhow!("Failed to create a CDN request client: {error}"))),
    };

    // A tracker for the completed block height.
    let completed_height: Arc<RwLock<u32>> = Arc::new(RwLock::new(start_height));
    // A tracker to indicate if the sync failed.
    let failed: Arc<RwLock<Option<anyhow::Error>>> = Default::default();

    // Start a timer.
    let timer = Instant::now();

    futures::stream::iter(cdn_range.clone().step_by(BLOCKS_PER_FILE as usize))
        .map(|start| {
            // Prepare the end height.
            let end = start + BLOCKS_PER_FILE;

            // If the sync *has not* failed, log the progress.
            let ctx = format!("blocks {start} to {end}");
            if failed.read().is_none() {
                debug!("Requesting {ctx} (of {cdn_end})");
            }

            // Download the blocks with an exponential backoff retry policy.
            let client_clone = client.clone();
            let base_url_clone = base_url.to_string();
            let failed_clone = failed.clone();
            handle_dispatch_error(move || {
                let ctx = ctx.clone();
                let client = client_clone.clone();
                let base_url = base_url_clone.clone();
                let failed = failed_clone.clone();
                async move {
                    // If the sync failed, return with an empty vector.
                    if failed.read().is_some() {
                        return std::future::ready(Ok(vec![])).await
                    }
                    // Prepare the URL.
                    let blocks_url = format!("{base_url}/{start}.{end}.blocks");
                    // Fetch the blocks.
                    let blocks: Vec<Block<N>> = cdn_get(client, &blocks_url, &ctx).await?;
                    // Return the blocks.
                    std::future::ready(Ok(blocks)).await
                }
            })
        })
        .buffered(128) // The number of concurrent requests.
        .for_each(|result| async {
            // If the sync previously failed, return early.
            if failed.read().is_some() {
                return;
            }

            // Unwrap the blocks.
            let mut blocks = match result {
                Ok(blocks) => blocks,
                Err(error) => {
                    failed.write().replace(error);
                    return;
                }
            };

            // Only retain blocks that are at or above the start height and below the end height.
            blocks.retain(|block| block.height() >= start_height && block.height() < end_height);

            #[cfg(debug_assertions)]
            // Ensure the blocks are in order by height.
            for (i, block) in blocks.iter().enumerate() {
                if i > 0 {
                    assert_eq!(block.height(), blocks[i - 1].height() + 1);
                }
            }

            // Use blocking tasks, as deserialization and adding blocks are expensive operations.
            let mut process_clone = process.clone();
            let cdn_range_clone = cdn_range.clone();
            let completed_height_clone = completed_height.clone();
            let failed_clone = failed.clone();
            let result = tokio::task::spawn_blocking(move || {
                // Fetch the last height in the blocks.
                let curr_height = blocks.last().map(|block| block.height()).unwrap_or(start_height);

                // Process each of the blocks.
                for block in blocks {
                    // Retrieve the block height.
                    let block_height = block.height();

                    // If the sync failed, set the failed flag, and return.
                    if let Err(error) = process_clone(block) {
                        let error = anyhow!("Failed to process block {block_height}: {error}");
                        failed_clone.write().replace(error);
                        return;
                    }

                    // On success, update the completed height.
                    *completed_height_clone.write() = block_height;
                }

                // Log the progress.
                log_progress::<BLOCKS_PER_FILE>(timer, curr_height, &cdn_range_clone, "block");
            }).await;

            // If the sync failed, set the failed flag.
            if let Err(error) = result {
                let error = anyhow!("Failed to process blocks: {error}");
                failed.write().replace(error);
            }
        })
        .await;

    // Retrieve the successfully completed height (does not include failed blocks).
    let completed = *completed_height.read();
    // Return the result.
    match Arc::try_unwrap(failed).unwrap().into_inner() {
        // If the sync failed, return the completed height along with the error.
        Some(error) => Err((completed, error)),
        // Otherwise, return the completed height.
        None => Ok(completed),
    }
}

/// Retrieves the CDN height with the given base URL.
///
/// Note: This function decrements the tip by a few blocks, to ensure the
/// tip is not on a block that is not yet available on the CDN.
async fn cdn_height<const BLOCKS_PER_FILE: u32>() -> Result<u32> {
    const BASE_URL: &str = "https://vm.aleo.org/api";

    // Create a request client.
    let client = match reqwest::Client::builder().build() {
        Ok(client) => client,
        Err(error) => bail!("Failed to create a CDN request client: {error}"),
    };
    // Prepare the URL.
    let height_url = format!("{BASE_URL}/testnet3/latest/height");
    // Send the request.
    let response = match client.get(height_url).send().await {
        Ok(response) => response,
        Err(error) => bail!("Failed to fetch the CDN height: {error}"),
    };
    // Parse the response.
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => bail!("Failed to parse the CDN height response: {error}"),
    };
    // Parse the tip.
    let tip = match text.parse::<u32>() {
        Ok(tip) => tip,
        Err(error) => bail!("Failed to parse the CDN tip: {error}"),
    };
    // Decrement the tip by a few blocks to ensure the CDN is caught up.
    let tip = tip.saturating_sub(10);
    // Round the tip down to the nearest multiple.
    Ok(tip - (tip % BLOCKS_PER_FILE))
}

/// Retrieves the objects from the CDN with the given URL.
async fn cdn_get<T: 'static + DeserializeOwned + Send>(client: Client, url: &str, ctx: &str) -> Result<T> {
    // Fetch the bytes from the given URL.
    let response = match client.get(url).send().await {
        Ok(response) => response,
        Err(error) => bail!("Failed to fetch {ctx}: {error}"),
    };
    // Parse the response.
    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => bail!("Failed to parse {ctx}: {error}"),
    };
    // Parse the objects.
    match tokio::task::spawn_blocking(move || bincode::deserialize::<T>(&bytes)).await {
        Ok(Ok(objects)) => Ok(objects),
        Ok(Err(error)) => bail!("Failed to deserialize {ctx}: {error}"),
        Err(error) => bail!("Failed to join task for {ctx}: {error}"),
    }
}

/// Logs the progress of the sync.
fn log_progress<const OBJECTS_PER_FILE: u32>(
    timer: Instant,
    current_index: u32,
    cdn_range: &Range<u32>,
    object_name: &str,
) {
    // Prepare the CDN start and end heights.
    let cdn_start = cdn_range.start;
    let cdn_end = cdn_range.end;
    // Compute the percentage completed.
    let percentage = current_index * 100 / cdn_end;
    // Compute the number of files processed so far.
    let num_files_done = 1 + (current_index - cdn_start) / OBJECTS_PER_FILE;
    // Compute the number of files remaining.
    let num_files_remaining = 1 + (cdn_end - current_index) / OBJECTS_PER_FILE;
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

/// Executes the given closure, with a backoff policy, and returns the result.
pub(crate) async fn handle_dispatch_error<'a, T, F>(func: impl Fn() -> F + 'a) -> anyhow::Result<T>
where
    F: Future<Output = Result<T, anyhow::Error>>,
{
    use backoff::{future::retry, ExponentialBackoff};

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

#[cfg(test)]
mod tests {
    use crate::{
        blocks::{cdn_get, cdn_height, handle_dispatch_error, log_progress, BLOCKS_PER_FILE},
        load_blocks,
    };
    use snarkvm::prelude::{Block, Testnet3};

    use anyhow::{anyhow, Result};
    use parking_lot::RwLock;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Instant,
    };

    type CurrentNetwork = Testnet3;

    const TEST_BASE_URL: &str = "https://testnet3.blocks.aleo.org/phase2";

    fn check_load_blocks(start: u32, end: Option<u32>, expected: usize) {
        let blocks = Arc::new(RwLock::new(Vec::new()));
        let blocks_clone = blocks.clone();
        let process = move |block: Block<CurrentNetwork>| {
            blocks_clone.write().push(block);
            Ok(())
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let completed_height = load_blocks(TEST_BASE_URL, start, end, process).await.unwrap();
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
        rt.block_on(async {
            let height = cdn_height::<BLOCKS_PER_FILE>().await.unwrap();
            assert!(height > 0);
        });
    }

    #[test]
    fn test_cdn_get() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = reqwest::Client::new();
            let height =
                cdn_get::<u32>(client, &format!("{TEST_BASE_URL}/testnet3/latest/height"), "height").await.unwrap();
            assert!(height > 0);
        });
    }

    #[test]
    fn test_log_progress() {
        // This test sanity checks that basic arithmetic is correct (i.e. no divide by zero, etc.).
        let timer = Instant::now();
        let cdn_range = &(0..100);
        let object_name = "blocks";
        log_progress::<10>(timer, 0, cdn_range, object_name);
        log_progress::<10>(timer, 10, cdn_range, object_name);
        log_progress::<10>(timer, 20, cdn_range, object_name);
        log_progress::<10>(timer, 30, cdn_range, object_name);
        log_progress::<10>(timer, 40, cdn_range, object_name);
        log_progress::<10>(timer, 50, cdn_range, object_name);
        log_progress::<10>(timer, 60, cdn_range, object_name);
        log_progress::<10>(timer, 70, cdn_range, object_name);
        log_progress::<10>(timer, 80, cdn_range, object_name);
        log_progress::<10>(timer, 90, cdn_range, object_name);
        log_progress::<10>(timer, 100, cdn_range, object_name);
    }

    #[test]
    fn test_handle_dispatch_error() {
        let counter = AtomicUsize::new(0);

        let result: Result<()> = tokio_test::block_on(handle_dispatch_error(|| async {
            counter.fetch_add(1, Ordering::SeqCst);
            Err(anyhow!("test error"))
        }));

        assert!(result.is_err());
        assert!(counter.load(Ordering::SeqCst) >= 10);
    }

    #[test]
    fn test_handle_dispatch_error_success() {
        let counter = AtomicUsize::new(0);

        let result = tokio_test::block_on(handle_dispatch_error(|| async {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(42)
        }));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
