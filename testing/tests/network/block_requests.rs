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

use snarkos::{helpers::handle_block_requests, Client, Environment};
use snarkos_testing::ClientNode;
use snarkvm::dpc::testnet2::Testnet2;

use rand::{thread_rng, Rng};

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

#[tokio::test]
async fn test_block_requests_case_1() {
    let rng = &mut thread_rng();

    // Start a snarkos node
    let client_node = ClientNode::default().await;
    let peers_router = client_node.server.peers().router();

    // Declare internal state.
    let current_block_height: u32 = rng.gen_range(1000..5000000);

    // Declare peer state.
    let peer_ip = Some(format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap());
    let peer_is_fork = None;
    let peer_maximum_block_height: u32 = rng.gen_range(1..current_block_height);
    let peer_first_deviating_locator = None;

    // Declare locator state.
    let maximum_common_ancestor = peer_maximum_block_height;

    // Determine if block requests or forking is required.
    let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
        current_block_height,
        peer_ip,
        peer_is_fork,
        peer_maximum_block_height,
        maximum_common_ancestor,
        peer_first_deviating_locator,
        &peers_router,
    )
    .await;

    // Validate the output.
    assert_eq!(result, None);
}

#[tokio::test]
async fn test_block_requests_case_2a() {
    let rng = &mut thread_rng();

    // Start a snarkos node
    let client_node = ClientNode::default().await;
    let peers_router = client_node.server.peers().router();

    // Declare internal state.
    let current_block_height: u32 = rng.gen_range(1..1000);

    // Declare peer state.
    let peer_ip = Some(format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap());
    let peer_is_fork = None;
    let peer_maximum_block_height: u32 = rng.gen_range(current_block_height..current_block_height * 2);

    // Declare locator state.
    let maximum_common_ancestor = current_block_height;
    let peer_first_deviating_locator = Some(maximum_common_ancestor + 1);

    // Determine if block requests or forking is required.
    let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
        current_block_height,
        peer_ip,
        peer_is_fork,
        peer_maximum_block_height,
        maximum_common_ancestor,
        peer_first_deviating_locator.as_ref(),
        &peers_router,
    )
    .await;

    // Validate the output.
    assert_eq!(result, None);
}

#[tokio::test]
async fn test_block_requests_case_2b() {
    let rng = &mut thread_rng();

    // Start a snarkos node
    let client_node = ClientNode::default().await;
    let peers_router = client_node.server.peers().router();

    // Declare internal state.
    let current_block_height: u32 = rng.gen_range(1..1000);

    // Declare peer state.
    let peer_ip = Some(format!("127.0.0.1:{}", rng.gen::<u16>()).parse().unwrap());
    let peer_is_fork = Some(false);
    let peer_maximum_block_height: u32 = rng.gen_range(current_block_height..current_block_height * 2);

    // Declare locator state.
    let maximum_common_ancestor = current_block_height;
    let peer_first_deviating_locator = Some(maximum_common_ancestor + 1);

    // Determine if block requests or forking is required.
    let result = handle_block_requests::<Testnet2, Client<Testnet2>>(
        current_block_height,
        peer_ip,
        peer_is_fork,
        peer_maximum_block_height,
        maximum_common_ancestor,
        peer_first_deviating_locator.as_ref(),
        &peers_router,
    )
    .await;

    // Validate the output.
    assert!(result.is_some());
    let (starting_block_height, end_block_height, requires_fork) = result.unwrap();

    let expected_number_of_block_requests =
        std::cmp::min(end_block_height - current_block_height, Client::<Testnet2>::MAXIMUM_BLOCK_REQUEST);
    let expected_end_block_height = current_block_height + expected_number_of_block_requests;

    assert_eq!(starting_block_height, current_block_height + 1);
    assert_eq!(end_block_height, expected_end_block_height);
    assert_eq!(requires_fork, false);
}

#[tokio::test]
async fn test_block_requests_case_2ca() {
    // TODO (raychu86): Implement this test.
}

#[tokio::test]
async fn test_block_requests_case_2cba() {
    // TODO (raychu86): Implement this test.
}

#[tokio::test]
async fn test_block_requests_case_2cbb() {
    // TODO (raychu86): Implement this test.
}
