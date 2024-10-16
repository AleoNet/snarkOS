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

#![recursion_limit = "256"]

#[allow(dead_code)]
mod common;
use common::{node::*, test_peer::TestPeer};

use snarkos_node::{Client, Prover, Validator};
use snarkos_node_router::Outbound;
use snarkos_node_tcp::P2P;
use snarkvm::prelude::{store::helpers::memory::ConsensusMemory, MainnetV0 as CurrentNetwork};

use pea2pea::Pea2Pea;

use std::{io, net::SocketAddr, time::Duration};
use tokio::time::sleep;

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

impl_connect!(Client, Prover, Validator);

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

mod client {
    // Initiator side (full node connects to synthetic peer).
    test_handshake! {
        client -> client,
        client -> validator,
        client -> prover
    }

    // Responder side (synthetic peer connects to full node).
    test_handshake! {
        client <- client,
        client <- validator,
        client <- prover
    }
}

mod prover {
    // Initiator side (full node connects to synthetic peer).
    test_handshake! {
        prover -> client,
        prover -> validator,
        prover -> prover
    }

    // Responder side (synthetic peer connects to full node).
    test_handshake! {
        prover <- client,
        prover <- validator,
        prover <- prover
    }
}

mod validator {
    // Initiator side (full node connects to synthetic peer).
    test_handshake! {
        validator -> client,
        validator -> validator,
        validator -> prover
    }

    // Responder side (synthetic peer connects to full node).
    test_handshake! {
        validator <- client,
        validator <- validator,
        validator <- prover
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn simultaneous_connection_attempt() {
    // common::initialise_logger(3);

    // Spin up 2 full nodes.
    let node1 = validator().await;
    let addr1 = node1.listening_addr();
    let node2 = validator().await;
    let addr2 = node2.listening_addr();

    // Prepare connection attempts.
    let node1_clone = node1.clone();
    let conn1 = tokio::spawn(async move {
        if let Some(conn_task) = node1_clone.router().connect(addr2) { conn_task.await.unwrap() } else { false }
    });
    let node2_clone = node2.clone();
    let conn2 = tokio::spawn(async move {
        if let Some(conn_task) = node2_clone.router().connect(addr1) { conn_task.await.unwrap() } else { false }
    });

    // Attempt to connect both nodes to one another at the same time.
    let (result1, result2) = tokio::join!(conn1, conn2);
    // A small anti-flakiness buffer.
    sleep(Duration::from_millis(200)).await;

    // Count connection successes.
    let mut successes = 0;
    if result1.unwrap() {
        successes += 1;
    }
    if result2.unwrap() {
        successes += 1;
    }

    // Record the number of connected peers for both nodes.
    let tcp_connected1 = node1.tcp().num_connected();
    let tcp_connected2 = node2.tcp().num_connected();
    let router_connected1 = node1.router().number_of_connected_peers();
    let router_connected2 = node2.router().number_of_connected_peers();

    // It's possible for both attempts to fail and that's ok; the important
    // thing is that at most a single connection is established in the end.
    assert!(successes <= 1);

    // If both attempts failed, all the counters should be 0; otherwise,
    // all should be 1.
    if successes == 0 {
        assert_eq!(tcp_connected1, 0);
        assert_eq!(tcp_connected2, 0);
        assert_eq!(router_connected1, 0);
        assert_eq!(router_connected2, 0);
    } else {
        assert_eq!(tcp_connected1, 1);
        assert_eq!(tcp_connected2, 1);
        assert_eq!(router_connected1, 1);
        assert_eq!(router_connected2, 1);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn duplicate_connection_attempts() {
    // common::initialise_logger(3);

    // Spin up 2 full nodes.
    let node1 = validator().await;
    let node2 = validator().await;
    let addr2 = node2.listening_addr();

    // Prepare connection attempts.
    let node1_clone = node1.clone();
    let conn1 = tokio::spawn(async move {
        if let Some(conn_task) = node1_clone.router().connect(addr2) { conn_task.await.unwrap() } else { false }
    });
    let node1_clone = node1.clone();
    let conn2 = tokio::spawn(async move {
        if let Some(conn_task) = node1_clone.router().connect(addr2) { conn_task.await.unwrap() } else { false }
    });
    let node1_clone = node1.clone();
    let conn3 = tokio::spawn(async move {
        if let Some(conn_task) = node1_clone.router().connect(addr2) { conn_task.await.unwrap() } else { false }
    });

    // Attempt to connect the 1st node to the other one several times at once.
    let (result1, result2, result3) = tokio::join!(conn1, conn2, conn3);
    // A small anti-flakiness buffer.
    sleep(Duration::from_millis(200)).await;

    // Count the successes.
    let mut successes = 0;
    if result1.unwrap() {
        successes += 1;
    }
    if result2.unwrap() {
        successes += 1;
    }
    if result3.unwrap() {
        successes += 1;
    }

    // Connection checks.
    assert_eq!(successes, 1);
    assert_eq!(node1.router().number_of_connected_peers(), 1);
    assert_eq!(node2.router().number_of_connected_peers(), 1);
}
