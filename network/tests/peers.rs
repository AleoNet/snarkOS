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

use snarkos_network::external::message::*;
use snarkos_testing::{
    network::{
        handshaken_node_and_peer,
        random_bound_address,
        read_header,
        read_payload,
        test_node,
        write_message_to_stream,
        TestSetup,
    },
    wait_until,
};

#[tokio::test]
async fn peer_initiator_side() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        min_peers: 2,
        ..Default::default()
    };
    let (node, mut peer_stream) = handshaken_node_and_peer(setup).await;

    // the buffer for peer's reads
    let mut peer_buf = [0u8; 64];

    // check if the peer has received the GetPeers message from the node
    let len = read_header(&mut peer_stream).await.unwrap().len();
    let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
    assert!(matches!(bincode::deserialize(&payload).unwrap(), Payload::GetPeers));

    // respond with a Peers message
    let (addr, _) = random_bound_address().await;
    let peers = Payload::Peers(vec![addr]);
    write_message_to_stream(peers, &mut peer_stream).await;

    // check the address has been added to the disconnected list in the peer book
    wait_until!(5, node.peers.is_disconnected(addr));
}

#[tokio::test]
async fn peer_responder_side() {
    let setup = TestSetup {
        consensus_setup: None,
        ..Default::default()
    };
    let (_node, mut peer_stream) = handshaken_node_and_peer(setup).await;

    // send GetPeers message
    write_message_to_stream(Payload::GetPeers, &mut peer_stream).await;

    // the buffer for peer's reads
    let mut peer_buf = [0u8; 64];

    // check if the peer has received the Peers message from the node
    // TODO(nkls): check the message contains a node, currently empty as there is no simple way to
    // insert a node into the peer book marked as connected other than spinning up another and
    // connecting them.
    let len = read_header(&mut peer_stream).await.unwrap().len();
    let payload = read_payload(&mut peer_stream, &mut peer_buf[..len]).await.unwrap();
    assert!(matches!(bincode::deserialize(&payload).unwrap(), Payload::Peers(..)));
}

#[tokio::test(flavor = "multi_thread")]
async fn triangle() {
    let setup = |bootnodes| TestSetup {
        consensus_setup: None,
        min_peers: 2,
        peer_sync_interval: 2,
        bootnodes,
        ..Default::default()
    };

    // Spin up and connect nodes A and B.
    let node_alice = test_node(setup(vec![])).await;
    let addr_alice = node_alice.local_address().unwrap();

    let node_bob = test_node(setup(vec![addr_alice.to_string()])).await;
    let addr_bob = node_bob.local_address().unwrap();

    //  Spin up node C and connect to B.
    let node_charlie = test_node(setup(vec![addr_bob.to_string()])).await;

    let triangle_is_formed = || {
        node_charlie.peers.is_connected(addr_alice)
            && node_alice.peers.number_of_connected_peers() == 2
            && node_bob.peers.number_of_connected_peers() == 2
            && node_charlie.peers.number_of_connected_peers() == 2
    };

    // Make sure C connects to A => peer propagation works.
    wait_until!(5, triangle_is_formed());
}
