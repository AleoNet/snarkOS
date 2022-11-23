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

mod common;

use snarkos_account::Account;
use snarkos_node::{Beacon, Validator};
use snarkvm::prelude::{ConsensusMemory, Testnet3 as CurrentNetwork};

use std::str::FromStr;

async fn beacon() -> Beacon<CurrentNetwork, ConsensusMemory<CurrentNetwork>> {
    Beacon::new(
        "127.0.0.1:0".parse().unwrap(),
        None,
        Account::<CurrentNetwork>::from_str("APrivateKey1zkp2oVPTci9kKcUprnbzMwq95Di1MQERpYBhEeqvkrDirK1").unwrap(),
        &[],
        None, // Should load the current network's genesis block.
        None, // No CDN.
        None,
    )
    .await
    .expect("couldn't create beacon instance")
}

async fn validator() -> Validator<CurrentNetwork, ConsensusMemory<CurrentNetwork>> {
    Validator::new(
        "127.0.0.1:0".parse().unwrap(),
        None,
        Account::<CurrentNetwork>::from_str("APrivateKey1zkp2oVPTci9kKcUprnbzMwq95Di1MQERpYBhEeqvkrDirK1").unwrap(),
        &[],
        None, // Should load the current network's genesis block.
        None, // No CDN.
        None,
    )
    .await
    .expect("couldn't create beacon instance")
}

macro_rules! test_handshake {
    ($($node_type:tt -> $peer_type:ident),*) => {
        mod handshake_initiator_side {
            use snarkos_node_tcp::P2P;

            $(
                #[tokio::test]
                async fn $peer_type () {

                    let node = $crate::$node_type().await;

                    // Spin up a test peer.
                    let peer = $crate::common::TestPeer::$peer_type().await;

                    // Verify the handshake works when the node initiates a connection with the peer.
                    assert!(
                        node.tcp().connect(peer.tcp().listening_addr().expect("node listener should exist")).await.is_ok()
                    );
                }

            )*
        }

    };

    ($($node_type:tt <- $peer_type:ident),*) => {
        mod handshake_responder_side {
            use snarkos_node_tcp::P2P;
            use snarkos_node_router::Outbound;

            $(
                #[tokio::test]
                async fn $peer_type () {

                    let node = $crate::$node_type().await;

                    // Spin up a test peer.
                    let peer = $crate::common::TestPeer::$peer_type().await;

                    // Verify the handshake works when the peer initiates a connection with the node.
                    assert!(
                        peer.tcp().connect(node.router().tcp().listening_addr().expect("node listener should exist")).await.is_ok()
                    );
                }

            )*
        }

    };
}

mod beacon {
    // Initiator side.
    test_handshake! {
        beacon -> beacon,
        beacon -> client,
        beacon -> validator,
        beacon -> prover
    }

    // Responder side.
    test_handshake! {
        beacon <- beacon,
        beacon <- client,
        beacon <- validator,
        beacon <- prover
    }
}

mod validator {
    // Initiator side.
    test_handshake! {
        validator -> beacon,
        validator -> client,
        validator -> validator,
        validator -> prover
    }

    // Responder side.
    test_handshake! {
        validator <- beacon,
        validator <- client,
        validator <- validator,
        validator <- prover
    }
}
