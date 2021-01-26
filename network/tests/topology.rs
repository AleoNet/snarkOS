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
        connect_nodes,
        handshaken_node_and_peer,
        random_bound_address,
        read_header,
        read_payload,
        write_message_to_stream,
        TestSetup,
        Topology,
    },
    wait_until,
};

#[tokio::test]
async fn line() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        ..Default::default()
    };
    let nodes = connect_nodes(10, setup, Topology::Line).await;

    // First and Last nodes should have 1 connected peer.
    wait_until!(5, nodes.first().unwrap().peers.number_of_connected_peers() == 1);
    assert_eq!(nodes.last().unwrap().peers.number_of_connected_peers(), 1);

    // All other nodes should have two.
    for i in 1..(nodes.len() - 1) {
        assert_eq!(nodes[i].peers.number_of_connected_peers(), 2);
    }
}

#[tokio::test]
async fn star() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        ..Default::default()
    };
    let nodes = connect_nodes(10, setup, Topology::Star).await;
    let core = nodes.first().unwrap();

    assert!(core.environment.is_bootnode());
    wait_until!(5, core.peers.number_of_connected_peers() == 9);
}
