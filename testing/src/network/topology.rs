// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use crate::network::{test_node, TestSetup};

use snarkos_network::Server;

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

pub async fn connect_nodes(n: u32, setup: TestSetup, topology: Topology) -> Vec<Server> {
    if n < 2 {
        unimplemented!();
    }

    match topology {
        Topology::Line | Topology::Ring | Topology::Mesh => unimplemented!(),
        Topology::Star => star_topology(n, setup).await,
    }
}

/// Starts n network nodes in a star topology.
///
/// The bootnode is at the center and is included in the total node count.
pub async fn star_topology(n: u32, setup: TestSetup) -> Vec<Server> {
    // Start the bootnode at the center of the star.
    let core_setup = TestSetup {
        is_bootnode: true,
        ..setup.clone()
    };
    let core = test_node(core_setup).await;
    let core_addr = core.local_address().unwrap();

    // Start the rest of the nodes with the core node as the bootnode.
    let mut nodes = vec![core];
    for _ in 1..n {
        let leaf_setup = TestSetup {
            bootnodes: vec![core_addr.to_string()],
            ..setup.clone()
        };

        let node = test_node(leaf_setup).await;
        nodes.push(node);
    }

    nodes
}
