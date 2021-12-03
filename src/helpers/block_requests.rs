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

use crate::Environment;
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
    latest_cumulative_weight: u128,
    maximal_peer: Option<SocketAddr>,
    maximal_peer_is_fork: Option<bool>,
    maximum_block_height: u32,
    maximum_cumulative_weight: u128,
    maximum_common_ancestor: u32,
    first_deviating_locator: Option<&u32>,
) -> Option<(u32, u32, bool)> {
    // Case 1 - Ensure the peer has a heavier canonical chain than this ledger.
    if latest_cumulative_weight >= maximum_cumulative_weight {
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
        let (latest_common_ancestor, ledger_requires_revert) =
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
                    // Revert if the latest block is not the maximum common ancestor.
                    let requires_revert = latest_block_height != maximum_common_ancestor;
                    (maximum_common_ancestor, requires_revert)
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
                        return None;
                    }
                    // Case 2(c)(b)(b) - You don't know if your real common ancestor is within `MAXIMUM_FORK_DEPTH`.
                    // Revert to the common ancestor anyways.
                    else {
                        info!("Discovered a potentially better canonical chain from {} with common ancestor {} and cumulative weight {}", peer_ip, maximum_common_ancestor, maximum_cumulative_weight);
                        (maximum_common_ancestor, true)
                    }
                }
            };

        // TODO (howardwu): Ensure the start <= end.
        // Determine the start and end block heights to request.
        let number_of_block_requests = std::cmp::min(maximum_block_height - latest_common_ancestor, E::MAXIMUM_BLOCK_REQUEST);
        let start_block_height = latest_common_ancestor + 1;
        let end_block_height = start_block_height + number_of_block_requests - 1;

        return Some((start_block_height, end_block_height, ledger_requires_revert));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;
    use snarkvm::dpc::testnet2::Testnet2;

    use rand::{thread_rng, Rng};

    const ITERATIONS: usize = 50;

    #[tokio::test]
    async fn test_block_requests_case_1() {
        // Case 1 - You are ahead of your peer: Do nothing

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let current_block_height: u32 = rng.gen_range(1000..5000000);
            let current_cumulative_weight: u128 = current_block_height as u128;

            // Declare peer state.
            let peer_ip = Some(format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap());
            let peer_is_fork = None;
            let peer_maximum_block_height: u32 = rng.gen_range(1..current_block_height);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;
            let peer_first_deviating_locator = None;

            // Declare locator state.
            let maximum_common_ancestor = peer_maximum_block_height;

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                current_block_height,
                current_cumulative_weight,
                peer_ip,
                peer_is_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator,
            )
            .await;

            // Validate the output.
            assert_eq!(result, None);
        }
    }

    #[tokio::test]
    async fn test_block_requests_case_2a() {
        // Case 2(a) -  You are behind your peer and `is_fork` is `None`:
        // Peer is being malicious or thinks you are ahead. Both are issues,
        // pick a different peer to sync with.

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let current_block_height: u32 = rng.gen_range(1..1000);
            let current_cumulative_weight: u128 = current_block_height as u128;

            // Declare peer state.
            let peer_ip = Some(format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap());
            let peer_is_fork = None;
            let peer_maximum_block_height: u32 = rng.gen_range(current_block_height..current_block_height * 2);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;

            // Declare locator state.
            let maximum_common_ancestor = current_block_height;
            let peer_first_deviating_locator = Some(maximum_common_ancestor + 1);

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                current_block_height,
                current_cumulative_weight,
                peer_ip,
                peer_is_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator.as_ref(),
            )
            .await;

            // Validate the output.
            assert_eq!(result, None);
        }
    }

    #[tokio::test]
    async fn test_block_requests_case_2b() {
        // Case 2(b) -  You are behind your peer and `is_fork` is `Some(false)`:
        // Request blocks from your latest state.

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let current_block_height: u32 = rng.gen_range(1..1000);
            let current_cumulative_weight: u128 = current_block_height as u128;

            // Declare peer state.
            let peer_ip = Some(format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap());
            let peer_is_fork = Some(false);
            let peer_maximum_block_height: u32 = rng.gen_range(current_block_height..current_block_height * 2);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;

            // Declare locator state.
            let maximum_common_ancestor = current_block_height;
            let peer_first_deviating_locator = Some(maximum_common_ancestor + 1);

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                current_block_height,
                current_cumulative_weight,
                peer_ip,
                peer_is_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator.as_ref(),
            )
            .await;

            // Validate the output.
            assert!(result.is_some());
            let (starting_block_height, end_block_height, requires_fork) = result.unwrap();

            let expected_number_of_block_requests =
                std::cmp::min(end_block_height - current_block_height, Client::<Testnet2>::MAXIMUM_BLOCK_REQUEST);
            let expected_end_block_height = current_block_height + expected_number_of_block_requests;

            assert_eq!(starting_block_height, maximum_common_ancestor + 1);
            assert_eq!(end_block_height, expected_end_block_height);
            assert_eq!(requires_fork, false);
        }
    }

    #[tokio::test]
    async fn test_block_requests_case_2ca() {
        // Case 2(c)(a) -  You are behind your peer, `is_fork` is `Some(true)`, and Common ancestor is within `MAXIMUM_FORK_DEPTH`:
        // Request blocks from your latest state.

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let current_block_height: u32 =
                rng.gen_range(Client::<Testnet2>::MAXIMUM_FORK_DEPTH + 1..Client::<Testnet2>::MAXIMUM_FORK_DEPTH * 2);
            let current_cumulative_weight: u128 = current_block_height as u128;

            // Declare peer state.
            let peer_ip = Some(format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap());
            let peer_is_fork = Some(true);

            // Generate a common ancestor within the maximum fork depth.
            let peer_maximum_block_height: u32 = rng.gen_range(current_block_height + 1..current_block_height * 2);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;

            // Declare locator state.
            let maximum_common_ancestor =
                rng.gen_range((current_block_height - Client::<Testnet2>::MAXIMUM_FORK_DEPTH)..current_block_height);
            let peer_first_deviating_locator = Some(rng.gen_range(maximum_common_ancestor + 1..current_block_height));

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                current_block_height,
                current_cumulative_weight,
                peer_ip,
                peer_is_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator.as_ref(),
            )
            .await;

            // Validate the output.
            assert!(result.is_some());
            let (starting_block_height, end_block_height, requires_fork) = result.unwrap();

            let expected_number_of_block_requests =
                std::cmp::min(end_block_height - starting_block_height, Client::<Testnet2>::MAXIMUM_BLOCK_REQUEST);
            let expected_end_block_height = starting_block_height + expected_number_of_block_requests;

            assert_eq!(starting_block_height, maximum_common_ancestor + 1);
            assert_eq!(end_block_height, expected_end_block_height);
            assert_eq!(requires_fork, true);
        }
    }

    #[tokio::test]
    async fn test_block_requests_case_2cba() {
        // Case 2(c)(b)(a) -  You are behind your peer, `is_fork` is `Some(true)`,
        //    Common ancestor is NOT within `MAXIMUM_FORK_DEPTH`, and you can calculate
        // that you are outside of the `MAXIMUM_FORK_DEPTH`:
        // Disconnect from peer.

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let current_block_height: u32 =
                rng.gen_range(Client::<Testnet2>::MAXIMUM_FORK_DEPTH + 1..Client::<Testnet2>::MAXIMUM_FORK_DEPTH * 2);
            let current_cumulative_weight: u128 = current_block_height as u128;

            // Declare peer state.
            let peer_ip = Some(format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap());
            let peer_is_fork = Some(true);

            // Generate a common ancestor within the maximum fork depth.
            let peer_maximum_block_height: u32 = rng.gen_range(current_block_height + 1..current_block_height * 2);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;

            // Declare locator state.
            let maximum_common_ancestor = rng.gen_range(0..(current_block_height - Client::<Testnet2>::MAXIMUM_FORK_DEPTH) / 2);
            let peer_first_deviating_locator =
                Some(rng.gen_range(maximum_common_ancestor + 1..current_block_height - Client::<Testnet2>::MAXIMUM_FORK_DEPTH));

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                current_block_height,
                current_cumulative_weight,
                peer_ip,
                peer_is_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator.as_ref(),
            )
            .await;

            // Validate the output.
            assert_eq!(result, None);
        }
    }

    #[tokio::test]
    async fn test_block_requests_case_2cbb() {
        // Case 2(c)(b)(a) -  You are behind your peer, `is_fork` is `Some(true)`,
        //    Common ancestor is NOT within `MAXIMUM_FORK_DEPTH`, and You don't know if
        // you are within the `MAXIMUM_FORK_DEPTH`:
        // Disconnect from peer.

        let rng = &mut thread_rng();

        for _ in 0..ITERATIONS {
            // Declare internal state.
            let current_block_height: u32 =
                rng.gen_range(Client::<Testnet2>::MAXIMUM_FORK_DEPTH + 1..Client::<Testnet2>::MAXIMUM_FORK_DEPTH * 2);
            let current_cumulative_weight: u128 = current_block_height as u128;

            // Declare peer state.
            let peer_ip = Some(format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap());
            let peer_is_fork = Some(true);

            // Generate a common ancestor within the maximum fork depth.
            let peer_maximum_block_height: u32 = rng.gen_range(current_block_height + 1..current_block_height * 2);
            let peer_maximum_cumulative_weight: u128 = peer_maximum_block_height as u128;

            // Declare locator state.
            let maximum_common_ancestor = rng.gen_range(0..current_block_height - Client::<Testnet2>::MAXIMUM_FORK_DEPTH);
            let peer_first_deviating_locator =
                Some(rng.gen_range(current_block_height - Client::<Testnet2>::MAXIMUM_FORK_DEPTH..current_block_height));

            // Determine if block requests or forking is required.
            let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
                current_block_height,
                current_cumulative_weight,
                peer_ip,
                peer_is_fork,
                peer_maximum_block_height,
                peer_maximum_cumulative_weight,
                maximum_common_ancestor,
                peer_first_deviating_locator.as_ref(),
            )
            .await;

            // Validate the output.
            assert!(result.is_some());
            let (starting_block_height, end_block_height, requires_fork) = result.unwrap();

            let expected_number_of_block_requests =
                std::cmp::min(end_block_height - starting_block_height, Client::<Testnet2>::MAXIMUM_BLOCK_REQUEST);
            let expected_end_block_height = starting_block_height + expected_number_of_block_requests;

            assert_eq!(starting_block_height, maximum_common_ancestor + 1);
            assert_eq!(end_block_height, expected_end_block_height);
            assert_eq!(requires_fork, true);
        }
    }
}
