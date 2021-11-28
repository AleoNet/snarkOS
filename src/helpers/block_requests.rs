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

use crate::{Environment, Message, PeersRequest, PeersRouter};
use snarkvm::dpc::prelude::*;

use std::net::SocketAddr;

///
/// Handle block requests. Returns the start/end block heights to request and if the ledger requires a fork.
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
pub async fn handle_block_requests<N: Network, E: Environment>(
    latest_block_height: u32,
    maximal_peer: Option<SocketAddr>,
    maximal_peer_is_fork: Option<bool>,
    maximum_block_height: u32,
    maximum_common_ancestor: u32,
    first_deviating_locator: Option<&u32>,
    peers_router: &PeersRouter<N, E>,
) -> Option<(u32, u32, bool)> {
    // Case 1 - Ensure the peer has a higher block height than this ledger.
    if latest_block_height >= maximum_block_height {
        return None;
    }

    // Case 2 - Proceed to send block requests, as the peer is ahead of this ledger.
    if let (Some(peer_ip), Some(is_fork)) = (maximal_peer, maximal_peer_is_fork) {
        // Ensure the latest common ancestor is not greater than the latest block request.
        if latest_block_height < maximum_common_ancestor {
            warn!(
                "The common ancestor {} cannot be greater than the latest block {}",
                maximum_common_ancestor, latest_block_height
            );
            return None;
        }

        // Determine the latest common ancestor.
        let (latest_common_ancestor, ledger_requires_fork) =
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
                // Case 2(c)(a) - If the common ancestor is within the fork range of this ledger,
                // proceed to switch to the fork.
                if latest_block_height.saturating_sub(maximum_common_ancestor) <= E::MAXIMUM_FORK_DEPTH
                {
                    info!("Found a longer chain from {} starting at block {}", peer_ip, maximum_common_ancestor);
                    // If the latest block is the same as the maximum common ancestor, do not revert.
                    if latest_block_height != maximum_common_ancestor {
                        return None;
                    }
                    (maximum_common_ancestor, true)
                }
                // Case 2(c)(b) - If the common ancestor is NOT within `MAXIMUM_FORK_DEPTH`.
                else {
                    // Ensure that the first deviating locator exists.
                    let first_deviating_locator = match first_deviating_locator {
                        Some(locator) => locator,
                        None => return None,
                    };

                    // Case 2(c)(b)(a) - Check if the real common ancestor is NOT within `MAXIMUM_FORK_DEPTH`.
                    // If this peer is outside of the fork range of this ledger, proceed to disconnect from the peer.
                    if latest_block_height.saturating_sub(*first_deviating_locator) >= E::MAXIMUM_FORK_DEPTH {
                        debug!("Peer {} is outside of the fork range of this ledger, disconnecting", peer_ip);
                        // Send a `Disconnect` message to the peer.
                        let request = PeersRequest::MessageSend(peer_ip, Message::Disconnect);
                        if let Err(error) = peers_router.send(request).await {
                            warn!("[Disconnect] {}", error);
                        }
                        return None;
                    }
                    // Case 2(c)(b)(b) - You don't know if your real common ancestor is within `MAXIMUM_FORK_DEPTH`.
                    // Revert to the common ancestor anyways.
                    else {
                        info!("Found a potentially longer chain from {} starting at block {}", peer_ip, maximum_common_ancestor);
                        (maximum_common_ancestor, true)
                    }
                }
            };

        // TODO (howardwu): Ensure the start <= end.
        // Determine the start and end block heights to request.
        let number_of_block_requests = std::cmp::min(maximum_block_height - latest_common_ancestor, E::MAXIMUM_BLOCK_REQUEST);
        let start_block_height = latest_common_ancestor + 1;
        let end_block_height = start_block_height + number_of_block_requests - 1;

        return Some((start_block_height, end_block_height, ledger_requires_fork));
    }

    None
}
