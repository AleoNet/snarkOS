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

use std::net::SocketAddr;

/// Checks if any of the peers are ahead and have a larger block height, if they are on a fork, and their block locators.
/// The maximum known block height and cumulative weight are tracked for the purposes of further operations.
pub fn find_maximal_peer<N: Network, E: Environment>(
    peers_state: &PeersState<N>,
    maximum_block_height: &mut u32,
    maximum_cumulative_weight: &mut u128,
) -> Option<(SocketAddr, bool, BlockLocators<N>)> {
    // Determine if the peers state has any sync nodes.
    // TODO: have nodes sync up to tip - 4096 with only sync nodes, then switch to syncing with the longest chain.
    let peers_contains_sync_node = false;
    // for ip in peers_state.keys() {
    //     peers_contains_sync_node |= sync_nodes.contains(ip);
    // }

    let mut maximal_peer = None;

    for (peer_ip, peer_state) in peers_state.iter() {
        // Only update the maximal peer if there are no sync nodes or the peer is a sync node.
        if !peers_contains_sync_node || E::sync_nodes().contains(peer_ip) {
            // Update the maximal peer state if the peer is ahead and the peer knows if you are a fork or not.
            // This accounts for (Case 1 and Case 2(a))
            if let Some((_, _, is_on_fork, block_height, block_locators)) = peer_state {
                // Retrieve the cumulative weight, defaulting to the block height if it does not exist.
                let cumulative_weight = match block_locators.get_cumulative_weight(*block_height) {
                    Some(cumulative_weight) => cumulative_weight,
                    None => *block_height as u128,
                };
                // If the cumulative weight is more, set this peer as the maximal peer.
                if cumulative_weight > *maximum_cumulative_weight && is_on_fork.is_some() {
                    maximal_peer = Some((*peer_ip, is_on_fork.unwrap(), block_locators.clone()));
                    *maximum_block_height = *block_height;
                    *maximum_cumulative_weight = cumulative_weight;
                }
            }
        }
    }

    maximal_peer
}

/// Returns the common ancestor and the first deviating locator (if it exists),
/// given the block locators of a peer. If the peer has invalid block locators, returns an error.
pub fn find_common_ancestor<N: Network>(canon: &LedgerState<N>, block_locators: &BlockLocators<N>) -> Result<(u32, Option<u32>), String> {
    // Determine the common ancestor block height between this ledger and the peer.
    let mut maximum_common_ancestor = 0;
    // Determine the first locator (smallest height) that does not exist in this ledger.
    let mut first_deviating_locator = None;

    for (block_height, (block_hash, _)) in block_locators.iter() {
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

/// A case annotation enum for the block request handler.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Case {
    /// Case 0 - The common ancestor is greater than the latest block height:
    ///     - Abort. This is an internal error that needs to be remedied promptly.
    Zero,
    /// Case 1 - You are ahead of your peer:
    ///     - Abort. There is no need to send block requests as you are ahead of the maximal peer.
    One,
    /// Case 2 - You are behind your peer:
    ///     Case 2(a) - `is_on_fork` is `None`:
    ///         - Abort. Peer is malicious or thinks you are ahead. Both are issues, pick a different peer to sync with.
    TwoA,
    /// Case 2 - You are behind your peer:
    ///     Case 2(b) - `is_on_fork` is `Some(false)`:
    ///         - Request blocks from your latest state
    TwoB,
    /// Case 2 - You are behind your peer:
    ///     Case 2(c) - `is_on_fork` is `Some(true)`:
    ///         Case 2(c)(a) - Common ancestor is within `ALEO_MAXIMUM_FORK_DEPTH`:
    ///              - Revert to common ancestor, and send block requests to sync.
    TwoCA,
    /// Case 2 - You are behind your peer:
    ///     Case 2(c) - `is_on_fork` is `Some(true)`:
    ///         Case 2(c)(b) - Common ancestor is NOT within `ALEO_MAXIMUM_FORK_DEPTH`, and the first deviating locator exists:
    ///              Case 2(c)(b)(a) - You can calculate that you are outside of the `ALEO_MAXIMUM_FORK_DEPTH`:
    ///                  - Disconnect from peer.
    TwoCBA,
    /// Case 2 - You are behind your peer:
    ///     Case 2(c) - `is_on_fork` is `Some(true)`:
    ///         Case 2(c)(b) - Common ancestor is NOT within `ALEO_MAXIMUM_FORK_DEPTH`, and the first deviating locator exists:
    ///              Case 2(c)(b)(b) - You don't know if you are within the `ALEO_MAXIMUM_FORK_DEPTH`:
    ///                  - Revert to most common ancestor and send block requests to sync.
    TwoCBB,
    /// Case 2 - You are behind your peer:
    ///     Case 2(c) - `is_on_fork` is `Some(true)`:
    ///         Case 2(c)(c) - Common ancestor is NOT within `ALEO_MAXIMUM_FORK_DEPTH`, and the first deviating locator is missing:
    ///              - Abort. Peer may be malicious as the first deviating locator must exist.
    TwoCC,
}

/// The successful outcome of a block request handler.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BlockRequestHandlerProceed {
    pub(crate) start_block_height: u32,
    pub(crate) end_block_height: u32,
    pub(crate) ledger_is_on_fork: bool,
}

/// The result of calling the block request handler.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BlockRequestHandler {
    Abort(Case),
    AbortAndDisconnect(Case, String),
    Proceed(Case, BlockRequestHandlerProceed),
}

///
/// Determines the appropriate block request update operation,
/// based on the cases as described in the `Case` enum.
///
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_block_requests<N: Network, E: Environment>(
    latest_block_height: u32,
    latest_cumulative_weight: u128,
    maximal_peer: SocketAddr,
    maximal_peer_is_on_fork: Option<bool>,
    maximum_block_height: u32,
    maximum_cumulative_weight: u128,
    maximum_common_ancestor: u32,
    first_deviating_locator: Option<u32>,
) -> BlockRequestHandler {
    // Case 0 - Ensure the latest common ancestor is not greater than the latest block request.
    if latest_block_height < maximum_common_ancestor {
        warn!(
            "Common ancestor {} cannot exceed the latest block {}",
            maximum_common_ancestor, latest_block_height
        );
        return BlockRequestHandler::Abort(Case::Zero);
    }

    // Case 1 - Ensure the peer has a heavier canonical chain than this ledger.
    if latest_cumulative_weight >= maximum_cumulative_weight {
        return BlockRequestHandler::Abort(Case::One);
    }

    // Case 2 - Prepare to send block requests, as the peer is ahead of this ledger.
    // Determine the latest common ancestor, and whether the ledger is on a fork & needs to revert.
    let (case, latest_common_ancestor, ledger_is_on_fork) =
        // Case 2(a) - Peer is malicious or thinks you are ahead. Both are issues, pick a different peer to sync with.
        if maximal_peer_is_on_fork.is_none() {
            return BlockRequestHandler::Abort(Case::TwoA);
        }
        // Case 2(b) - This ledger is not a fork of the peer, it is on the same canon chain.
        else if let Some(false) = maximal_peer_is_on_fork {
            // Continue to sync from the latest block height of this ledger, if the peer is honest.
            match first_deviating_locator.is_none() {
                true => (Case::TwoB, maximum_common_ancestor, false),
                false => (Case::TwoB, latest_block_height, false),
            }
        }
        // Case 2(c) - This ledger is on a fork of the peer.
        else {
            // Case 2(c)(a) - If the common ancestor is within the fork range of this ledger, proceed to switch to the fork.
            if latest_block_height.saturating_sub(maximum_common_ancestor) <= N::ALEO_MAXIMUM_FORK_DEPTH {
                info!("Discovered a canonical chain from {} with common ancestor {} and cumulative weight {}", maximal_peer, maximum_common_ancestor, maximum_cumulative_weight);
                // If the latest block is the same as the maximum common ancestor, do not revert.
                (Case::TwoCA, maximum_common_ancestor, latest_block_height != maximum_common_ancestor)
            }
            // Case 2(c)(b) - If the common ancestor is NOT within `ALEO_MAXIMUM_FORK_DEPTH`.
            else if let Some(first_deviating_locator) = first_deviating_locator {
                // Case 2(c)(b)(a) - Check if the real common ancestor is NOT within `ALEO_MAXIMUM_FORK_DEPTH`.
                // If this peer is outside of the fork range of this ledger, proceed to disconnect from the peer.
                if latest_block_height.saturating_sub(first_deviating_locator) >= N::ALEO_MAXIMUM_FORK_DEPTH {
                    debug!("Peer {} exceeded the permitted fork range, disconnecting", maximal_peer);
                    return BlockRequestHandler::AbortAndDisconnect(Case::TwoCBA, "exceeded fork range".into());
                }
                // Case 2(c)(b)(b) - You don't know if your real common ancestor is within `ALEO_MAXIMUM_FORK_DEPTH`.
                // Revert to the common ancestor anyways.
                else {
                    info!("Discovered a potentially better canonical chain from {} with common ancestor {} and cumulative weight {}", maximal_peer, maximum_common_ancestor, maximum_cumulative_weight);
                    (Case::TwoCBB, maximum_common_ancestor, true)
                }
            }
            // Case 2(c)(c) - The first deviating locator does not exist; abort.
            else {
                warn!("Peer {} is missing first deviating locator", maximal_peer);
                return BlockRequestHandler::Abort(Case::TwoCC);
            }
        };

    // TODO (howardwu): Ensure the start <= end.
    // Determine the start and end block heights to request.
    let number_of_block_requests = std::cmp::min(maximum_block_height - latest_common_ancestor, E::MAXIMUM_BLOCK_REQUEST);
    let start_block_height = latest_common_ancestor + 1;
    let end_block_height = start_block_height + number_of_block_requests - 1;

    BlockRequestHandler::Proceed(case, BlockRequestHandlerProceed {
        start_block_height,
        end_block_height,
        ledger_is_on_fork,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;
    use snarkvm::dpc::testnet2::Testnet2;

    use rand::{thread_rng, Rng};

    const ITERATIONS: usize = 50;

    #[tokio::test]
    async fn test_block_requests_case_0() {
        // Case 1 - You are ahead of your peer: Do nothing

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let latest_block_height: u32 = rng.gen_range(1000..5000000);
            let latest_cumulative_weight: u128 = latest_block_height as u128;

            // Declare peer state.
            let peer_ip = format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap();
            let peer_is_on_fork = None;
            let peer_maximum_block_height: u32 = latest_block_height + 1;
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;
            let peer_first_deviating_locator = None;

            // Declare locator state.
            let maximum_common_ancestor = peer_maximum_block_height;

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                latest_block_height,
                latest_cumulative_weight,
                peer_ip,
                peer_is_on_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator,
            );

            // Validate the output.
            assert_eq!(result, BlockRequestHandler::Abort(Case::Zero));
        }
    }

    #[tokio::test]
    async fn test_block_requests_case_1() {
        // Case 1 - You are ahead of your peer: Do nothing

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let latest_block_height: u32 = rng.gen_range(1000..5000000);
            let latest_cumulative_weight: u128 = latest_block_height as u128;

            // Declare peer state.
            let peer_ip = format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap();
            let peer_is_on_fork = None;
            let peer_maximum_block_height: u32 = rng.gen_range(1..latest_block_height);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;
            let peer_first_deviating_locator = None;

            // Declare locator state.
            let maximum_common_ancestor = peer_maximum_block_height;

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                latest_block_height,
                latest_cumulative_weight,
                peer_ip,
                peer_is_on_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator,
            );

            // Validate the output.
            assert_eq!(result, BlockRequestHandler::Abort(Case::One));
        }
    }

    #[tokio::test]
    async fn test_block_requests_case_2a() {
        // Case 2(a) - You are behind your peer and `is_on_fork` is `None`:
        // Peer is malicious or thinks you are ahead. Both are issues, pick a different peer to sync with.

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let latest_block_height: u32 = rng.gen_range(1..1000);
            let latest_cumulative_weight: u128 = latest_block_height as u128;

            // Declare peer state.
            let peer_ip = format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap();
            let peer_is_on_fork = None;
            let peer_maximum_block_height: u32 = rng.gen_range(latest_block_height + 1..(latest_block_height + 1) * 2);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;

            // Declare locator state.
            let maximum_common_ancestor = latest_block_height;
            let peer_first_deviating_locator = Some(maximum_common_ancestor + 1);

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                latest_block_height,
                latest_cumulative_weight,
                peer_ip,
                peer_is_on_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator,
            );

            // Validate the output.
            assert_eq!(result, BlockRequestHandler::Abort(Case::TwoA));
        }
    }

    #[tokio::test]
    async fn test_block_requests_case_2b() {
        // Case 2(b) - You are behind your peer and `is_on_fork` is `Some(false)`:
        // Request blocks from your latest state.

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let latest_block_height: u32 = rng.gen_range(1..1000);
            let latest_cumulative_weight: u128 = latest_block_height as u128;

            // Declare peer state.
            let peer_ip = format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap();
            let peer_is_on_fork = Some(false);
            let peer_maximum_block_height: u32 = rng.gen_range(latest_block_height + 1..(latest_block_height + 1) * 2);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;

            // Declare locator state.
            let maximum_common_ancestor = latest_block_height;
            let peer_first_deviating_locator = Some(maximum_common_ancestor + 1);

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                latest_block_height,
                latest_cumulative_weight,
                peer_ip,
                peer_is_on_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator,
            );

            let expected_number_of_block_requests = std::cmp::min(
                peer_maximum_block_height - latest_block_height,
                Client::<Testnet2>::MAXIMUM_BLOCK_REQUEST,
            );
            let expected_start_block_height = latest_block_height + 1;
            let expected_end_block_height = expected_start_block_height + expected_number_of_block_requests - 1;

            // Validate the output.
            assert_eq!(
                result,
                BlockRequestHandler::Proceed(Case::TwoB, BlockRequestHandlerProceed {
                    start_block_height: expected_start_block_height,
                    end_block_height: expected_end_block_height,
                    ledger_is_on_fork: false,
                })
            );
        }
    }

    #[tokio::test]
    async fn test_block_requests_case_2ca() {
        // Case 2(c)(a) - You are behind your peer, `is_on_fork` is `Some(true)`, and common ancestor is within `ALEO_MAXIMUM_FORK_DEPTH`:
        // Request blocks from your latest state.

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let latest_block_height: u32 =
                rng.gen_range(Testnet2::ALEO_MAXIMUM_FORK_DEPTH + 1..(Testnet2::ALEO_MAXIMUM_FORK_DEPTH + 1) * 2);
            let latest_cumulative_weight: u128 = latest_block_height as u128;

            // Declare peer state.
            let peer_ip = format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap();
            let peer_is_on_fork = Some(true);

            // Generate a common ancestor within the maximum fork depth.
            let peer_maximum_block_height: u32 = rng.gen_range(latest_block_height + 1..(latest_block_height + 1) * 2);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;

            // Declare locator state.
            let maximum_common_ancestor =
                rng.gen_range(latest_block_height.saturating_sub(Testnet2::ALEO_MAXIMUM_FORK_DEPTH)..latest_block_height);
            let peer_first_deviating_locator = Some(rng.gen_range(maximum_common_ancestor + 1..latest_block_height));

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                latest_block_height,
                latest_cumulative_weight,
                peer_ip,
                peer_is_on_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator,
            );

            let expected_number_of_block_requests = std::cmp::min(
                peer_maximum_block_height - maximum_common_ancestor,
                Client::<Testnet2>::MAXIMUM_BLOCK_REQUEST,
            );
            let expected_start_block_height = maximum_common_ancestor + 1;
            let expected_end_block_height = expected_start_block_height + expected_number_of_block_requests - 1;

            // Validate the output.
            assert_eq!(
                result,
                BlockRequestHandler::Proceed(Case::TwoCA, BlockRequestHandlerProceed {
                    start_block_height: expected_start_block_height,
                    end_block_height: expected_end_block_height,
                    ledger_is_on_fork: true,
                })
            );
        }
    }

    #[tokio::test]
    async fn test_block_requests_case_2cba() {
        // Case 2(c)(b)(a) - You are behind your peer, `is_on_fork` is `Some(true)`,
        //    Common ancestor is NOT within `ALEO_MAXIMUM_FORK_DEPTH`, and you can calculate that you are outside of the `ALEO_MAXIMUM_FORK_DEPTH`:
        // Disconnect from peer.

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let latest_block_height: u32 =
                rng.gen_range(Testnet2::ALEO_MAXIMUM_FORK_DEPTH + 2..(Testnet2::ALEO_MAXIMUM_FORK_DEPTH + 2) * 2);
            let latest_cumulative_weight: u128 = latest_block_height as u128;

            // Declare peer state.
            let peer_ip = format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap();
            let peer_is_on_fork = Some(true);

            // Generate a common ancestor within the maximum fork depth.
            let peer_maximum_block_height: u32 = rng.gen_range(latest_block_height + 1..(latest_block_height + 1) * 2);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;

            // Declare locator state.
            let maximum_common_ancestor = rng.gen_range(0..latest_block_height.saturating_sub(Testnet2::ALEO_MAXIMUM_FORK_DEPTH) / 2);
            let peer_first_deviating_locator =
                Some(rng.gen_range(maximum_common_ancestor + 1..latest_block_height.saturating_sub(Testnet2::ALEO_MAXIMUM_FORK_DEPTH)));

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                latest_block_height,
                latest_cumulative_weight,
                peer_ip,
                peer_is_on_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator,
            );

            // Validate the output.
            assert_eq!(
                result,
                BlockRequestHandler::AbortAndDisconnect(Case::TwoCBA, "exceeded fork range".into())
            );
        }
    }

    #[tokio::test]
    async fn test_block_requests_case_2cbb() {
        // Case 2(c)(b)(b) - You are behind your peer, `is_on_fork` is `Some(true)`,
        //    Common ancestor is NOT within `ALEO_MAXIMUM_FORK_DEPTH`, and you don't know if you are within the `ALEO_MAXIMUM_FORK_DEPTH`::
        // Revert to most common ancestor and send block requests to sync.

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let latest_block_height: u32 =
                rng.gen_range(Testnet2::ALEO_MAXIMUM_FORK_DEPTH + 1..(Testnet2::ALEO_MAXIMUM_FORK_DEPTH + 1) * 2);
            let latest_cumulative_weight: u128 = latest_block_height as u128;

            // Declare peer state.
            let peer_ip = format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap();
            let peer_is_on_fork = Some(true);

            // Generate a common ancestor within the maximum fork depth.
            let peer_maximum_block_height: u32 = rng.gen_range(latest_block_height + 1..(latest_block_height + 1) * 2);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;

            // Declare locator state.
            let maximum_common_ancestor = rng.gen_range(0..latest_block_height.saturating_sub(Testnet2::ALEO_MAXIMUM_FORK_DEPTH));
            let peer_first_deviating_locator =
                Some(rng.gen_range(latest_block_height.saturating_sub(Testnet2::ALEO_MAXIMUM_FORK_DEPTH)..latest_block_height));

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                latest_block_height,
                latest_cumulative_weight,
                peer_ip,
                peer_is_on_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator,
            );

            let expected_number_of_block_requests = std::cmp::min(
                peer_maximum_block_height - maximum_common_ancestor,
                Client::<Testnet2>::MAXIMUM_BLOCK_REQUEST,
            );
            let expected_start_block_height = maximum_common_ancestor + 1;
            let expected_end_block_height = expected_start_block_height + expected_number_of_block_requests - 1;

            // Validate the output.
            assert_eq!(
                result,
                BlockRequestHandler::Proceed(Case::TwoCBB, BlockRequestHandlerProceed {
                    start_block_height: expected_start_block_height,
                    end_block_height: expected_end_block_height,
                    ledger_is_on_fork: true,
                })
            );
        }
    }

    #[tokio::test]
    async fn test_block_requests_case_2cc() {
        // Case 2(c)(b)(b) - You are behind your peer, `is_on_fork` is `Some(true)`,
        //    Common ancestor is NOT within `ALEO_MAXIMUM_FORK_DEPTH`, and you don't know if you are within the `ALEO_MAXIMUM_FORK_DEPTH`::
        // Revert to most common ancestor and send block requests to sync.

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let latest_block_height: u32 =
                rng.gen_range(Testnet2::ALEO_MAXIMUM_FORK_DEPTH + 1..(Testnet2::ALEO_MAXIMUM_FORK_DEPTH + 1) * 2);
            let latest_cumulative_weight: u128 = latest_block_height as u128;

            // Declare peer state.
            let peer_ip = format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap();
            let peer_is_on_fork = Some(true);

            // Generate a common ancestor within the maximum fork depth.
            let peer_maximum_block_height: u32 = rng.gen_range(latest_block_height + 1..(latest_block_height + 1) * 2);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;

            // Declare locator state.
            let maximum_common_ancestor = rng.gen_range(0..latest_block_height.saturating_sub(Testnet2::ALEO_MAXIMUM_FORK_DEPTH));
            let peer_first_deviating_locator = None;

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                latest_block_height,
                latest_cumulative_weight,
                peer_ip,
                peer_is_on_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator,
            );

            // Validate the output.
            assert_eq!(result, BlockRequestHandler::Abort(Case::TwoCC));
        }
    }
}
