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
    network::{handshaken_node_and_peer, random_bound_address, test_node, TestSetup},
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
async fn triangle() {
    let setup = |bootnodes| TestSetup {
        consensus_setup: None,
        min_peers: 2,
        peer_sync_interval: 1,
        bootnodes,
        ..Default::default()
    };

    // Spin up and connect nodes A and B.
    let node_alice = test_node(setup(vec![])).await;
    let addr_alice = node_alice.local_address().unwrap();

    // wait a few ms so that nodes don't try to connect into one another simultaneously
    sleep(Duration::from_millis(10)).await;

    let node_bob = test_node(setup(vec![addr_alice.to_string()])).await;
    let addr_bob = node_bob.local_address().unwrap();

    sleep(Duration::from_millis(10)).await;

    //  Spin up node C and connect to B.
    let node_charlie = test_node(setup(vec![addr_bob.to_string()])).await;

    let triangle_is_formed = || {
        node_charlie.peer_book.is_connected(addr_alice)
            && node_alice.peer_book.get_active_peer_count() == 2
            && node_bob.peer_book.get_active_peer_count() == 2
            && node_charlie.peer_book.get_active_peer_count() == 2
    };

    // Make sure C connects to A => peer propagation works.
    wait_until!(5, triangle_is_formed());
}
