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

use std::{collections::HashSet, net::SocketAddr};

pub enum Topology {
    /// Each node - except the last one - connects to the next one in a linear fashion.
    Line,
    /// Like the `Line`, but the last node connects to the first one, forming a ring.
    Ring,
    /// All the nodes become connected to one another, forming a full mesh.
    Mesh,
    /// The first node is the central one (the hub); all the other nodes connect to it.
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
        Topology::Star => star(nodes).await,
    }
}

/// Connects the network nodes in a line topology.
async fn line(nodes: &mut Vec<Node>) {
    let mut prev_node: Option<SocketAddr> = None;

    // Start each node with the previous as a bootnode.
    for mut node in nodes {
        let bootnodes = match prev_node {
            Some(addr) => vec![addr],
            None => vec![],
        };

        node.environment.bootnodes = bootnodes;

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
    nodes.last_mut().unwrap().environment.bootnodes.push(first_addr);
}

/// Connects the network nodes in a mesh topology.
async fn mesh(nodes: &mut Vec<Node>) {
    let mut connected_pairs = HashSet::new();

    for i in 0..nodes.len() {
        for j in 0..nodes.len() {
            if i != j && connected_pairs.insert((i, j)) && connected_pairs.insert((j, i)) {
                let addr = nodes[j].local_address().unwrap();
                nodes[i].environment.bootnodes.push(addr);
            }
        }
    }
}

/// Connects thr network nodes in a star topology.
async fn star(nodes: &mut Vec<Node>) {
    // Setup the hub.
    let hub_address = nodes.first().unwrap().local_address().unwrap();

    // Start the rest of the nodes with the core node as the bootnode.
    let bootnodes = vec![hub_address];
    for i in 1..nodes.len() {
        nodes[i].environment.bootnodes = bootnodes.clone();
    }
}
