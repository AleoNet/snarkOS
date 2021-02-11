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

use snarkos_network::Node;
use snarkos_testing::{
    network::{
        test_environment,
        topology::{connect_nodes, Topology},
        TestSetup,
    },
    wait_until,
};

const N: usize = 10;

async fn test_nodes() -> Vec<Node> {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        ..Default::default()
    };

    let mut nodes = vec![];

    for _ in 0..N {
        let environment = test_environment(setup.clone());
        let mut node = Node::new(environment).await.unwrap();

        node.establish_address().await.unwrap();
        nodes.push(node);
    }

    nodes
}

async fn start_nodes(nodes: &Vec<Node>) {
    for node in nodes {
        node.start_services().await;
    }
}

#[tokio::test]
async fn line() {
    let mut nodes = test_nodes().await;
    connect_nodes(&mut nodes, Topology::Line).await;
    start_nodes(&nodes).await;

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

#[tokio::test]
async fn ring() {
    let mut nodes = test_nodes().await;
    connect_nodes(&mut nodes, Topology::Ring).await;
    start_nodes(&nodes).await;

    for node in &nodes {
        wait_until!(5, node.peer_book.read().number_of_connected_peers() == 2);
    }
}

#[tokio::test]
async fn mesh() {
    let mut nodes = test_nodes().await;
    connect_nodes(&mut nodes, Topology::Mesh).await;
    start_nodes(&nodes).await;

    for node in &nodes {
        wait_until!(5, node.peer_book.read().number_of_connected_peers() as usize == N - 1);
    }
}

#[tokio::test]
async fn star() {
    let mut nodes = test_nodes().await;
    connect_nodes(&mut nodes, Topology::Star).await;
    start_nodes(&nodes).await;

    let hub = nodes.first().unwrap();
    wait_until!(5, hub.peer_book.read().number_of_connected_peers() as usize == N - 1);
}
