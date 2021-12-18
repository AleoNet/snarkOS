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

use crate::network::ledger::PeersState;
use snarkos_storage::{BlockLocators, LedgerState};
use snarkvm::dpc::prelude::*;

use rand::{rngs::OsRng, seq::SliceRandom};
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
    let mut candidates = Vec::with_capacity(8);
    let mut maybe_maximum_block_height = *maximum_block_height;
    let mut maybe_maximum_cumulative_weight = *maximum_cumulative_weight;

    for (peer_ip, peer_state) in peers_state.iter() {
        // Only update the maximal peer if there are no sync nodes or the peer is a sync node.
        if !peers_contains_sync_node || sync_nodes.contains(peer_ip) {
            // Update the maximal peer state if the peer is ahead and the peer knows if you are a fork or not.
            // This accounts for (Case 1 and Case 2(a))
            // Since the peers with is_fork == None will not influence the result,
            // we simply let the pattern matching do the stuff here
            if let Some((_, _, Some(is_fork), block_height, block_locators)) = peer_state {
                let block_height = *block_height;
                // Retrieve the cumulative weight, defaulting to the block height if it does not exist.
                let cumulative_weight = match block_locators.get_cumulative_weight(block_height) {
                    Some(cumulative_weight) => cumulative_weight,
                    None => block_height as u128,
                };

                // too light or too late
                if cumulative_weight < maybe_maximum_cumulative_weight
                    || (cumulative_weight == maybe_maximum_cumulative_weight && block_height > maybe_maximum_block_height)
                {
                    continue;
                }

                // find a better one, clear & replace
                if cumulative_weight > maybe_maximum_cumulative_weight
                    || (cumulative_weight == maybe_maximum_cumulative_weight && block_height < maybe_maximum_block_height)
                {
                    trace!(w = %cumulative_weight, h = block_height, "replace candidates");
                    candidates.clear();
                    candidates.push((peer_ip, is_fork, block_locators));
                    maybe_maximum_cumulative_weight = cumulative_weight;
                    maybe_maximum_block_height = block_height;
                    continue;
                }

                // another candidate
                candidates.push((peer_ip, is_fork, block_locators));
            }
        }
    }

    let candidates_count = candidates.len();
    candidates.choose(&mut OsRng).map(|(&peer_ip, &is_fork, block_locators)| {
        // we actually have one candidate here
        // set the weight & height, and return what we want
        *maximum_block_height = maybe_maximum_block_height;
        *maximum_cumulative_weight = maybe_maximum_cumulative_weight;
        trace!(
            %peer_ip,
            is_fork,
            candidates_count,
            weight_target = %maybe_maximum_cumulative_weight,
            height_target = maybe_maximum_block_height,
            "sync target chosen"
        );
        (peer_ip, is_fork, (*block_locators).clone())
    })
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
