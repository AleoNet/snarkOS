// Copyright (C) 2019-2023 Aleo Systems Inc.
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

#![recursion_limit = "256"]

#[allow(dead_code)]
mod common;
use common::{node::*, test_peer::TestPeer};

// Macro to simply construct disconnect cases.
// Syntax:
// - (full_node |> test_peer): full node disconnects from the synthetic test peer.
// - (full_node <| test_peer): synthetic test peer disconnects from the full node.
//
// Test naming: full_node::handshake_<node or peer>_side::test_peer.
macro_rules! test_disconnect {
    ($node_type:ident, $peer_type:ident, $node_disconnects:expr, $($attr:meta)?) => {
        #[tokio::test]
        $(#[$attr])?
        async fn $peer_type() {
            use deadline::deadline;
            use pea2pea::Pea2Pea;
            #[allow(unused)]
            use snarkos_node_router::Outbound;
            use snarkos_node_tcp::P2P;
            use std::time::Duration;

            // $crate::common::initialise_logger(2);

            // Spin up a full node.
            let node = $crate::$node_type().await;

            // Spin up a test peer (synthetic node).
            let peer = $crate::TestPeer::$peer_type().await;
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

            // Disconnect.
            if $node_disconnects {
                node.router().disconnect(node.tcp().connected_addrs()[0]).await.unwrap();
            } else {
                peer.node().disconnect(peer.node().connected_addrs()[0]).await;
            }

            // Check the peer counts have been updated.
            let node_clone = node.clone();
            deadline!(Duration::from_secs(5), move || node_clone.router().number_of_connected_peers() == 0);
            deadline!(Duration::from_secs(5), move || node.tcp().num_connected() == 0);
            deadline!(Duration::from_secs(5), move || peer.node().num_connected() == 0);

        }
    };

    // Node side disconnect.
    ($($node_type:ident |> $peer_type:ident $(= $attr:meta)?),*) => {
        mod disconnect_node_side {
            $(
                test_disconnect!($node_type, $peer_type, true, $($attr)?);
            )*
        }
    };

    // Peer side disconnect.
    ($($node_type:ident <| $peer_type:ident $(= $attr:meta)?),*) => {
        mod disconnect_peer_side {
            $(
                test_disconnect!($node_type, $peer_type, false, $($attr)?);
            )*
        }
    };
}

mod beacon {
    // Full node disconnects from synthetic peer.
    test_disconnect! {
        beacon |> beacon = should_panic,
        beacon |> client,
        beacon |> validator,
        beacon |> prover
    }

    // Synthetic peer disconnects from the full node.
    test_disconnect! {
        beacon <| beacon = should_panic,
        beacon <| client,
        beacon <| validator,
        beacon <| prover
    }
}

mod client {
    // Full node disconnects from synthetic peer.
    test_disconnect! {
        client |> beacon = should_panic,
        client |> client,
        client |> validator,
        client |> prover
    }

    // Synthetic peer disconnects from the full node.
    test_disconnect! {
        client <| beacon = should_panic,
        client <| client,
        client <| validator,
        client <| prover
    }
}

mod prover {
    // Full node disconnects from synthetic peer.
    test_disconnect! {
        prover |> beacon = should_panic,
        prover |> client,
        prover |> validator,
        prover |> prover
    }

    // Synthetic peer disconnects from the full node.
    test_disconnect! {
        prover <| beacon = should_panic,
        prover <| client,
        prover <| validator,
        prover <| prover
    }
}

mod validator {
    // Full node disconnects from synthetic peer.
    test_disconnect! {
        validator |> beacon = should_panic,
        validator |> client,
        validator |> validator,
        validator |> prover
    }

    // Synthetic peer disconnects from the full node.
    test_disconnect! {
        validator <| beacon = should_panic,
        validator <| client,
        validator <| validator,
        validator <| prover
    }
}
