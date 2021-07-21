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

use std::time::Duration;

use snarkos_network::message::*;
use snarkos_testing::{
    network::{handshaken_node_and_peer, random_bound_address, started_test_node, TestSetup},
    wait_until,
};
use tokio::time::sleep;

#[tokio::test]
async fn peer_initiator_side() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        min_peers: 2,
        ..Default::default()
    };
    let (node, mut peer) = handshaken_node_and_peer(setup).await;

    // check if the peer has received the GetPeers message from the node
    let payload = peer.read_payload().await.unwrap();
    assert!(matches!(payload, Payload::GetPeers));

    // check if the peer has received an automatic Ping message from the node
    let payload = peer.read_payload().await.unwrap();
    assert!(matches!(payload, Payload::Ping(..)));

    // respond with a Peers message
    let (addr, _) = random_bound_address().await;
    peer.write_message(&Payload::Peers(vec![addr])).await;

    // check the address has been added to the disconnected list in the peer book
    wait_until!(5, node.peer_book.is_disconnected(addr));
}

#[tokio::test]
async fn peer_responder_side() {
    let setup = TestSetup {
        consensus_setup: None,
        ..Default::default()
    };
    let (_node, mut peer) = handshaken_node_and_peer(setup).await;

    // check if the peer has received an automatic Ping message from the node
    let payload = peer.read_payload().await.unwrap();
    assert!(matches!(payload, Payload::Ping(..)));

    // send GetPeers message
    peer.write_message(&Payload::GetPeers).await;

    // check if the peer has received the Peers message from the node
    let payload = peer.read_payload().await.unwrap();
    assert!(matches!(payload, Payload::Peers(..)));
}

#[tokio::test(flavor = "multi_thread")]
async fn bootnode_peer_propagation() {
    let setup = |is_bootnode, bootnodes| TestSetup {
        consensus_setup: None,
        min_peers: 2,
        peer_sync_interval: 1,
        is_bootnode,
        bootnodes,
        ..Default::default()
    };

    // Spin up and connect nodes A and B.
    let node_alice = started_test_node(setup(true, vec![])).await;
    let addr_alice = node_alice.local_address().unwrap();

    // Connect B to A.
    let node_bob = started_test_node(setup(false, vec![addr_alice.to_string()])).await;

    // Sleep to avoid C and B trying to simultaneously connect to each other.
    sleep(Duration::from_millis(100)).await;

    // Connect C to A.
    let node_charlie = started_test_node(setup(false, vec![addr_alice.to_string()])).await;

    let triangle_is_formed = || {
        node_charlie.peer_book.is_connected(addr_alice)
            && node_alice.peer_book.get_active_peer_count() == 2
            && node_bob.peer_book.get_active_peer_count() == 2
            && node_charlie.peer_book.get_active_peer_count() == 2
    };

    // Make sure B and C connect => bootnode propagates peers (without `is_routable` check in this
    // case).
    wait_until!(5, triangle_is_formed());
}
