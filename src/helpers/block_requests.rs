// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use crate::{network::ledger::PeersState, Environment};
use snarkos_storage::{BlockLocators, LedgerState};
use snarkvm::dpc::prelude::*;

use std::{collections::HashSet, net::SocketAddr};

/// Checks if any of the peers are ahead and have a larger block height, if they are on a fork, and their block locators.
/// The maximum known block height and cumulative weight are tracked for the purposes of further operations.
pub fn find_maximal_peer<N: Network>(
    peers_state: &PeersState<N>,
    sync_nodes: &HashSet<SocketAddr>,
    peers_contains_sync_node: bool,
    maximum_block_height: &mut u32,
    maximum_cumulative_weight: &mut u128,
) -> Option<(SocketAddr, bool, BlockLocators<N>)> {
    let mut maximal_peer = None;

    for (peer_ip, peer_state) in peers_state.iter() {
        // Only update the maximal peer if there are no sync nodes or the peer is a sync node.
        if !peers_contains_sync_node || sync_nodes.contains(peer_ip) {
            // Update the maximal peer state if the peer is ahead and the peer knows if you are a fork or not.
            // This accounts for (Case 1 and Case 2(a))
            if let Some((_, _, is_fork, block_height, block_locators)) = peer_state {
                // Retrieve the cumulative weight, defaulting to the block height if it does not exist.
                let cumulative_weight = match block_locators.get_cumulative_weight(*block_height) {
                    Some(cumulative_weight) => cumulative_weight,
                    None => *block_height as u128,
                };
                // If the cumulative weight is more, set this peer as the maximal peer.
                if cumulative_weight > *maximum_cumulative_weight && is_fork.is_some() {
                    maximal_peer = Some((*peer_ip, is_fork.unwrap(), block_locators.clone()));
                    *maximum_block_height = *block_height;
                    *maximum_cumulative_weight = cumulative_weight;
                }
            }
        }
    }

    maximal_peer
}

/// Verify the integrity of the block hashes sent by the peer.
/// Returns the maximum common ancestor and the first deviating locator (if any), or potentially an error containing a mismatch.
pub fn verify_block_hashes<N: Network>(
    canon: &LedgerState<N>,
    maximum_block_locators: &BlockLocators<N>,
) -> Result<(u32, Option<u32>), String> {
    // Determine the common ancestor block height between this ledger and the peer.
    let mut maximum_common_ancestor = 0;
    // Determine the first locator (smallest height) that does not exist in this ledger.
    let mut first_deviating_locator = None;

    for (block_height, (block_hash, _)) in maximum_block_locators.iter() {
        // Ensure the block hash corresponds with the block height, if the block hash exists in this ledger.
        if let Ok(expected_block_height) = canon.get_block_height(block_hash) {
            if expected_block_height != *block_height {
                let error = format!("Invalid block height {} for block hash {}", expected_block_height, block_hash);
                return Err(error);
            } else {
                // Update the common ancestor, as this block hash exists in this ledger.
                if expected_block_height > maximum_common_ancestor {
                    maximum_common_ancestor = expected_block_height;
                }
            }
        } else {
            // Update the first deviating locator.
            match first_deviating_locator {
                None => first_deviating_locator = Some(*block_height),
                Some(saved_height) => {
                    if *block_height < saved_height {
                        first_deviating_locator = Some(*block_height);
                    }
                }
            }
        }
    }

    Ok((maximum_common_ancestor, first_deviating_locator))
}

/// The successful outcome of a block request handler.
pub struct BlockRequestHandlerSuccess {
    pub start_block_height: u32,
    pub end_block_height: u32,
    pub ledger_is_on_fork: bool,
}

/// The result of calling the block request handler.
pub enum BlockRequestHandler {
    Success(BlockRequestHandlerSuccess),
    Abort,
    AbortAndDisconnect(String),
}

///
/// Determines the appropriate block request update operation, based on the following cases:
///
/// Case 1 - You are ahead of your peer:
///     - Do nothing
/// Case 2 - You are behind your peer:
///     Case 2(a) - `is_fork` is `None`:
///         - Peer is being malicious or thinks you are ahead. Both are issues,
///           pick a different peer to sync with.
///     Case 2(b) - `is_fork` is `Some(false)`:
///         - Request blocks from your latest state
///     Case 2(c) - `is_fork` is `Some(true)`:
///             Case 2(c)(a) - Common ancestor is within `MAXIMUM_FORK_DEPTH`:
///                  - Revert to common ancestor, and send block requests to sync.
///             Case 2(c)(b) - Common ancestor is NOT within `MAXIMUM_FORK_DEPTH`:
///                  Case 2(c)(b)(a) - You can calculate that you are outside of the `MAXIMUM_FORK_DEPTH`:
///                      - Disconnect from peer.
///                  Case 2(c)(b)(b) - You don't know if you are within the `MAXIMUM_FORK_DEPTH`:
///                      - Revert to most common ancestor and send block requests to sync.
///
pub fn handle_block_requests<E: Environment, N: Network>(
    latest_block_height: u32,
    latest_cumulative_weight: u128,
    peer_ip: SocketAddr,
    is_fork: bool,
    maximum_block_height: u32,
    maximum_cumulative_weight: u128,
    maximum_common_ancestor: u32,
    first_deviating_locator: Option<u32>,
) -> BlockRequestHandler {
    // Case 1 - Ensure the peer has a heavier canonical chain than this ledger.
    if latest_cumulative_weight >= maximum_cumulative_weight {
        return BlockRequestHandler::Abort;
    }

    // Ensure the latest common ancestor is not greater than the latest block request.
    if latest_block_height < maximum_common_ancestor {
        warn!(
            "The common ancestor {} cannot be greater than the latest block {}",
            maximum_common_ancestor, latest_block_height
        );
        return BlockRequestHandler::Abort;
    }

    // Determine the latest common ancestor, and whether the ledger is on a fork & needs to revert.
    let (latest_common_ancestor, ledger_is_on_fork) =
        // Case 2(b) - This ledger is not a fork of the peer, it is on the same canon chain.
        if !is_fork {
            // Continue to sync from the latest block height of this ledger, if the peer is honest.
            match first_deviating_locator.is_none() {
                true => (maximum_common_ancestor, false),
                false => (latest_block_height, false),
            }
        }
        // Case 2(c) - This ledger is on a fork of the peer.
        else {
            // Case 2(c)(a) - If the common ancestor is within the fork range of this ledger, proceed to switch to the fork.
            if latest_block_height.saturating_sub(maximum_common_ancestor) <= E::MAXIMUM_FORK_DEPTH {
                info!("Discovered a canonical chain from {} with common ancestor {} and cumulative weight {}", peer_ip, maximum_common_ancestor, maximum_cumulative_weight);
                // If the latest block is the same as the maximum common ancestor, do not revert.
                (maximum_common_ancestor, latest_block_height != maximum_common_ancestor)
            }
            // Case 2(c)(b) - If the common ancestor is NOT within `MAXIMUM_FORK_DEPTH`.
            else if let Some(first_deviating_locator) = first_deviating_locator {
                // Case 2(c)(b)(a) - Check if the real common ancestor is NOT within `MAXIMUM_FORK_DEPTH`.
                // If this peer is outside of the fork range of this ledger, proceed to disconnect from the peer.
                if latest_block_height.saturating_sub(first_deviating_locator) >= E::MAXIMUM_FORK_DEPTH {
                    debug!("Peer {} exceeded the permitted fork range, disconnecting", peer_ip);
                    return BlockRequestHandler::AbortAndDisconnect("exceeded fork range".into());
                }
                // Case 2(c)(b)(b) - You don't know if your real common ancestor is within `MAXIMUM_FORK_DEPTH`.
                // Revert to the common ancestor anyways.
                else {
                    info!("Discovered a potentially better canonical chain from {} with common ancestor {} and cumulative weight {}", peer_ip, maximum_common_ancestor, maximum_cumulative_weight);
                    (maximum_common_ancestor, true)
                }
            }
            // The first deviating locator does not exist; abort.
            else {
                warn!("Peer {} is missing first deviating locator", peer_ip);
                return BlockRequestHandler::Abort;
            }
        };

    // TODO (howardwu): Ensure the start <= end.
    // Determine the start and end block heights to request.
    let number_of_block_requests = std::cmp::min(maximum_block_height - latest_common_ancestor, E::MAXIMUM_BLOCK_REQUEST);
    let start_block_height = latest_common_ancestor + 1;
    let end_block_height = start_block_height + number_of_block_requests - 1;

    BlockRequestHandler::Success(BlockRequestHandlerSuccess {
        start_block_height,
        end_block_height,
        ledger_is_on_fork,
    })
}
