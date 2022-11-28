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

#![recursion_limit = "256"]

#[allow(dead_code)]
mod common;
use common::{node::*, test_peer::TestPeer};

macro_rules! test_disconnect {
    ($node_type:ident, $peer_type:ident, $node_disconnects:expr) => {
        #[tokio::test]
        async fn $peer_type() {
            use deadline::deadline;
            use pea2pea::Pea2Pea;
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
            node.router().connect(peer_addr);

            // Check the peer counts.
            let node_clone = node.clone();
            deadline!(Duration::from_secs(1), move || node_clone.router().number_of_connected_peers() == 1);
            let node_clone = node.clone();
            deadline!(Duration::from_secs(1), move || node_clone.tcp().num_connected() == 1);
            let peer_clone = peer.clone();
            deadline!(Duration::from_secs(1), move || peer_clone.node().num_connected() == 1);

            // Disconnect.
            if $node_disconnects {
                node.router().disconnect(node.tcp().connected_addrs()[0]);
            } else {
                peer.node().disconnect(peer.node().connected_addrs()[0]).await;
            }

            // Check the peer counts have been updated.
            let node_clone = node.clone();
            deadline!(Duration::from_secs(1), move || node_clone.router().number_of_connected_peers() == 0);
            deadline!(Duration::from_secs(1), move || node.tcp().num_connected() == 0);
            deadline!(Duration::from_secs(1), move || peer.node().num_connected() == 0);

        }
    };

    ($($node_type:ident |> $peer_type:ident),*) => {
        mod disconnect_node_side {
            $(
                test_disconnect!($node_type, $peer_type, true);
            )*
        }
    };

    ($($node_type:ident <| $peer_type:ident),*) => {
        mod disconnect_peer_side {
            $(
                test_disconnect!($node_type, $peer_type, false);
            )*
        }
    };
}

mod beacon {
    // Full node disconnects from synthetic peer.
    test_disconnect! {
        beacon |> beacon,
        beacon |> client,
        beacon |> validator,
        beacon |> prover
    }

    // Synthetic peer disconnects from the full node.
    test_disconnect! {
        beacon <| beacon,
        beacon <| client,
        beacon <| validator,
        beacon <| prover
    }
}

mod client {
    // Full node disconnects from synthetic peer.
    test_disconnect! {
        client |> beacon,
        client |> client,
        client |> validator,
        client |> prover
    }

    // Synthetic peer disconnects from the full node.
    test_disconnect! {
        client <| beacon,
        client <| client,
        client <| validator,
        client <| prover
    }
}

mod prover {
    // Full node disconnects from synthetic peer.
    test_disconnect! {
        prover |> beacon,
        prover |> client,
        prover |> validator,
        prover |> prover
    }

    // Synthetic peer disconnects from the full node.
    test_disconnect! {
        prover <| beacon,
        prover <| client,
        prover <| validator,
        prover <| prover
    }
}

mod validator {
    // Full node disconnects from synthetic peer.
    // validator |> beacon,
    test_disconnect! {
        validator |> client,
        validator |> validator,
        validator |> prover
    }

    // Synthetic peer disconnects from the full node.
    // validator <| beacon,
    test_disconnect! {
        validator <| client,
        validator <| validator,
        validator <| prover
    }
}
