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
use snarkos_storage::BlockLocators;
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
