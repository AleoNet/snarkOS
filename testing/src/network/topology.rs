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

use std::{net::SocketAddr, sync::Arc};

pub enum Topology {
    /// Each node - except the last one - connects to the next one in a linear fashion.
    Line,
    /// Like the `Line`, but the last node connects to the first one, forming a ring.
    Ring,
    /// All the nodes become connected to one another, forming a mesh.
    Mesh,
    /// The first node is the hub; all the other nodes connect to it.
    Star,
}

/// Connects the nodes in a given `Topology`.
///
/// This function assumes the nodes have an established address but don't have their services
/// started yet, as it uses the bootnodes to establish the connections between nodes.
///
/// When connecting in a `Star`, the first node in the `nodes` will be used as the hub.
pub async fn connect_nodes(nodes: &mut Vec<Node>, topology: Topology) {
    if nodes.len() < 2 {
        panic!("Can't connect less than two nodes");
    }

    match topology {
        Topology::Line => line(nodes).await,
        Topology::Ring => ring(nodes).await,
        Topology::Mesh => mesh(nodes).await,
        Topology::Star => star(nodes),
    }
}

/// Connects the network nodes in a line topology.
async fn line(nodes: &mut Vec<Node>) {
    let mut prev_node: Option<SocketAddr> = None;

    // Start each node with the previous as a bootnode.
    for node in nodes {
        if let Some(addr) = prev_node {
            node.connect_to_addresses(&[addr]).await;
        };

        // Assumes the node has an established address.
        prev_node = node.local_address();
    }
}

/// Connects the network nodes in a ring topology.
async fn ring(nodes: &mut Vec<Node>) {
    // Set the nodes up in a line.
    line(nodes).await;

    // Connect the first to the last.
    let first_addr = nodes.first().unwrap().local_address().unwrap();
    nodes.last().unwrap().connect_to_addresses(&[first_addr]).await;
}

/// Connects the network nodes in a mesh topology. The inital peers are selected at random based on the
/// minimum number of connected peers value.
async fn mesh(nodes: &mut Vec<Node>) {
    let local_addresses: Vec<SocketAddr> = nodes.iter().map(|node| node.local_address().unwrap()).collect();

    for node in nodes {
        use rand::seq::SliceRandom;
        let random_addrs: Vec<SocketAddr> = local_addresses
            .choose_multiple(
                &mut rand::thread_rng(),
                node.config.minimum_number_of_connected_peers().into(),
            )
            .copied()
            .collect();
        node.connect_to_addresses(&random_addrs).await;
    }
}

/// Connects the network nodes in a star topology.
fn star(nodes: &mut Vec<Node>) {
    // Setup the hub.
    let hub_address = nodes.first().unwrap().local_address().unwrap();

    // Start the rest of the nodes with the core node as the bootnode.
    let bootnodes = vec![hub_address];
    for node in nodes.iter_mut().skip(1) {
        node.config.bootnodes.store(Arc::new(bootnodes.clone()));
    }
}
