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

use snarkos_network::{message::*, Node};
use snarkos_testing::{
    network::{
        handshaken_node_and_peer,
        random_bound_address,
        read_header,
        read_payload,
        test_environment,
        topology::{connect_nodes, Topology},
        write_message_to_stream,
        TestSetup,
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

    let mut nodes = vec![];

    for _ in 0..10 {
        let environment = test_environment(setup.clone());
        let mut node = Node::new(environment).await.unwrap();

        node.establish_address().await.unwrap();
        nodes.push(node);
    }

    connect_nodes(&mut nodes, Topology::Line).await;

    for node in &nodes {
        node.start_services().await;
    }

    // First and Last nodes should have 1 connected peer.
    wait_until!(
        5,
        nodes.first().unwrap().peer_book.read().number_of_connected_peers() == 1
    );
    wait_until!(
        5,
        nodes.last().unwrap().peer_book.read().number_of_connected_peers() == 1
    );

    // All other nodes should have two.
    for i in 1..(nodes.len() - 1) {
        wait_until!(5, nodes[i].peer_book.read().number_of_connected_peers() == 2);
    }
}

// #[tokio::test]
// async fn star() {
//     let setup = TestSetup {
//         consensus_setup: None,
//         peer_sync_interval: 2,
//         ..Default::default()
//     };
//     let nodes = connect_nodes(10, setup, Topology::Star).await;
//     let core = nodes.first().unwrap();
//
//     assert!(core.environment.is_bootnode());
//     wait_until!(5, core.peer_book.read().number_of_connected_peers() == 9);
// }
