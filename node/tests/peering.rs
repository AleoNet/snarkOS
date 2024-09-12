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

#[allow(dead_code)]
mod common;
use common::test_peer::TestPeer;

use snarkos_node_router::{
    messages::{Message, PeerResponse},
    Outbound,
};
use snarkos_node_tcp::P2P;

use deadline::deadline;
use paste::paste;
use pea2pea::{protocols::Writing, Pea2Pea};
use std::time::Duration;

macro_rules! test_reject_unsolicited_peer_response {
    ($($node_type:ident),*) => {
        $(
            paste! {
                #[tokio::test]
                async fn [<$node_type _rejects_unsolicited_peer_response>]() {
                    // Spin up a full node.
                    let node = $crate::common::node::$node_type().await;

                    // Spin up a test peer (synthetic node), it doesn't really matter what type it is.
                    let peer = TestPeer::validator().await;
                    let peer_addr = peer.node().listening_addr().unwrap();

                    // Connect the node to the test peer.
                    node.router().connect(peer_addr).unwrap().await.unwrap();

                    // Check the peer counts.
                    let node_clone = node.clone();
                    deadline!(Duration::from_secs(5), move || node_clone.router().number_of_connected_peers() == 1);
                    let node_clone = node.clone();
                    deadline!(Duration::from_secs(5), move || node_clone.tcp().num_connected() == 1);
                    let peer_clone = peer.clone();
                    deadline!(Duration::from_secs(5), move || peer_clone.node().num_connected() == 1);

                    // Check the candidate peers.
                    assert_eq!(node.router().number_of_candidate_peers(), 0);

                    let peers = vec!["1.1.1.1:1111".parse().unwrap(), "2.2.2.2:2222".parse().unwrap()];

                    // Send a `PeerResponse` to the node.
                    assert!(
                        peer.unicast(
                            *peer.node().connected_addrs().first().unwrap(),
                            Message::PeerResponse(PeerResponse { peers: peers.clone() })
                        )
                        .is_ok()
                    );

                    // Wait for the peer to be disconnected for a protocol violation.
                    let node_clone = node.clone();
                    deadline!(Duration::from_secs(5), move || node_clone.router().number_of_connected_peers() == 0);

                    // Make sure the sent addresses weren't inserted in the candidate peers.
                    for peer in peers {
                        assert!(!node.router().candidate_peers().contains(&peer));
                    }
                }
            }
        )*
    };
}

test_reject_unsolicited_peer_response!(client, prover, validator);
