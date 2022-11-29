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

use snarkos_node::{Beacon, Client, Prover, Validator};
use snarkos_node_tcp::P2P;
use snarkvm::prelude::{ConsensusMemory, Testnet3 as CurrentNetwork};

use pea2pea::Pea2Pea;

use std::{io, net::SocketAddr};

// Trait to unify Pea2Pea and P2P traits.
#[async_trait::async_trait]
trait Connect {
    fn listening_addr(&self) -> SocketAddr;

    async fn connect(&self, target: SocketAddr) -> io::Result<()>;
}

// Implement the `Connect` trait for each node type.
macro_rules! impl_connect {
    ($($node_type:ident),*) => {
        $(
            #[async_trait::async_trait]
            impl Connect for $node_type<CurrentNetwork, ConsensusMemory<CurrentNetwork>> {
                fn listening_addr(&self) -> SocketAddr {
                    self.tcp().listening_addr().expect("node listener should exist")
                }

                async fn connect(&self, target: SocketAddr) -> io::Result<()>
                where
                    Self: P2P,
                {
                    self.tcp().connect(target).await
                }
            }
        )*
    };
}

impl_connect!(Beacon, Client, Prover, Validator);

// Implement the `Connect` trait for the test peer.
#[async_trait::async_trait]
impl Connect for TestPeer
where
    Self: Pea2Pea,
{
    fn listening_addr(&self) -> SocketAddr {
        self.node().listening_addr().expect("node listener should exist")
    }

    async fn connect(&self, target: SocketAddr) -> io::Result<()> {
        self.node().connect(target).await
    }
}

/* Test case */

// Asserts a successful connection was created from initiator to responder.
async fn assert_connect<T, U>(initiator: T, responder: U)
where
    T: Connect,
    U: Connect,
{
    initiator.connect(responder.listening_addr()).await.unwrap()
}

// Macro to simply construct handshake cases.
// Syntax:
// - (full_node -> test_peer): full node initiates a handshake to the test peer (synthetic node).
// - (full_node <- test_peer): full node receives a handshake initiated by the test peer.
//
// Test naming: full_node::handshake_<initiator or responder>_side::test_peer.
macro_rules! test_handshake {
    ($node_type:ident, $peer_type:ident, $is_initiator:expr, $($attr:meta)?) => {
        #[tokio::test]
        $(#[$attr])?
        async fn $peer_type() {
            // $crate::common::initialise_logger(2);

            // Spin up a full node.
            let node = $crate::$node_type().await;

            // Spin up a test peer (synthetic node).
            let peer = $crate::common::test_peer::TestPeer::$peer_type().await;

            // Sets up the connection direction as described above.
            if $is_initiator {
                $crate::assert_connect(node, peer).await;
            } else {
                $crate::assert_connect(peer, node).await;
            };
        }
    };

    // Initiator side.
    ($($node_type:ident -> $peer_type:ident $(= $attr:meta)?),*) => {
        mod handshake_initiator_side {
            $(
                test_handshake!($node_type, $peer_type, true, $($attr)?);
            )*
        }

    };

    // Responder side.
    ($($node_type:ident <- $peer_type:ident $(= $attr:meta)?),*) => {
        mod handshake_responder_side {
            $(
                test_handshake!($node_type, $peer_type, false, $($attr)?);
            )*
        }

    };
}

mod beacon {
    // Initiator side (full node connects to synthetic peer).
    test_handshake! {
        beacon -> beacon = should_panic,
        beacon -> client,
        beacon -> validator,
        beacon -> prover
    }

    // Responder side (synthetic peer connects to full node).
    test_handshake! {
        beacon <- beacon = should_panic,
        beacon <- client,
        beacon <- validator,
        beacon <- prover
    }
}

mod client {
    // Initiator side (full node connects to synthetic peer).
    test_handshake! {
        client -> beacon = should_panic,
        client -> client,
        client -> validator,
        client -> prover
    }

    // Responder side (synthetic peer connects to full node).
    test_handshake! {
        client <- beacon = should_panic,
        client <- client,
        client <- validator,
        client <- prover
    }
}

mod prover {
    // Initiator side (full node connects to synthetic peer).
    test_handshake! {
        prover -> beacon = should_panic,
        prover -> client,
        prover -> validator,
        prover -> prover
    }

    // Responder side (synthetic peer connects to full node).
    test_handshake! {
        prover <- beacon = should_panic,
        prover <- client,
        prover <- validator,
        prover <- prover
    }
}

mod validator {
    // Initiator side (full node connects to synthetic peer).
    test_handshake! {
        validator -> beacon = should_panic,
        validator -> client,
        validator -> validator,
        validator -> prover
    }

    // Responder side (synthetic peer connects to full node).
    test_handshake! {
        validator <- beacon = should_panic,
        validator <- client,
        validator <- validator,
        validator <- prover
    }
}
