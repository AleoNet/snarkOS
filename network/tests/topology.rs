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

use parking_lot::RwLock;
use snarkos_network::Node;
use snarkos_testing::{
    network::{
        test_environment,
        test_node,
        topology::{connect_nodes, Topology},
        TestSetup,
    },
    wait_until,
};
use std::sync::Arc;

const N: usize = 3;

async fn test_nodes(n: usize, setup: TestSetup) -> Vec<Node> {
    let mut nodes = vec![];

    for _ in 0..n {
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
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
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
#[ignore]
async fn line_degeneration() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        min_peers: (N / 2) as u16,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Line).await;
    start_nodes(&nodes).await;

    let density = || {
        let connections = total_connection_count(&nodes);
        network_density(N as f64, connections as f64)
    };
    wait_until!(5, density() >= 0.5);
    assert!(degree_centrality_delta(&nodes) as f64 <= 0.3 * N as f64);
}

#[tokio::test]
async fn ring() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Ring).await;
    start_nodes(&nodes).await;

    for node in &nodes {
        wait_until!(5, node.peer_book.read().number_of_connected_peers() == 2);
    }
}

#[tokio::test]
#[ignore]
async fn ring_degeneration() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        min_peers: (N / 2) as u16,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Ring).await;
    start_nodes(&nodes).await;

    let density = || {
        let connections = total_connection_count(&nodes);
        network_density(N as f64, connections as f64)
    };
    wait_until!(5, density() >= 0.5);
    assert!(degree_centrality_delta(&nodes) as f64 <= 0.3 * N as f64);
}

#[tokio::test]
async fn mesh() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Mesh).await;
    start_nodes(&nodes).await;

    for node in &nodes {
        wait_until!(5, node.peer_book.read().number_of_connected_peers() as usize == N - 1);
    }
}

#[tokio::test]
async fn star() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Star).await;
    start_nodes(&nodes).await;

    let hub = nodes.first().unwrap();
    wait_until!(5, hub.peer_book.read().number_of_connected_peers() as usize == N - 1);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn star_degeneration() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        min_peers: (N / 2) as u16,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Star).await;
    start_nodes(&nodes).await;

    let density = || {
        let connections = total_connection_count(&nodes);
        network_density(N as f64, connections as f64)
    };
    wait_until!(5, density() >= 0.5);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn binary_star_contact() {
    let filter = tracing_subscriber::EnvFilter::from_default_env().add_directive("tokio_reactor=off".parse().unwrap());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // 1. pass N nodes through a bootstrapper
    // 2. pass K nodes through a different bootstrapper (totally separate networks)
    // 3. introduce a node that is "fed" the list of random nodes from the N and K sets
    // 4. check out the end topology

    // Setup the bootnodes for each star topology.
    let bootnode_setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        min_peers: N as u16,
        is_bootnode: true,
        ..Default::default()
    };
    let environment_a = test_environment(bootnode_setup.clone());
    let environment_b = test_environment(bootnode_setup);
    let mut bootnode_a = Node::new(environment_a).await.unwrap();
    let mut bootnode_b = Node::new(environment_b).await.unwrap();

    bootnode_a.establish_address().await.unwrap();
    bootnode_b.establish_address().await.unwrap();

    let ba = bootnode_a.local_address().unwrap().to_string();
    let bb = bootnode_b.local_address().unwrap().to_string();

    // Create the nodes to be used as the leafs in the stars.
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        min_peers: (N / 2) as u16,
        ..Default::default()
    };
    let mut star_a_nodes = test_nodes(N - 1, setup.clone()).await;
    let mut star_b_nodes = test_nodes(N - 1, setup).await;

    // Insert the bootnodes at the begining of the node lists.
    star_a_nodes.insert(0, bootnode_a);
    star_b_nodes.insert(0, bootnode_b);

    // Create the star topologies.
    connect_nodes(&mut star_a_nodes, Topology::Star).await;
    connect_nodes(&mut star_b_nodes, Topology::Star).await;

    // Start the services.
    start_nodes(&star_a_nodes).await;
    start_nodes(&star_b_nodes).await;

    // Measure the initial density once the topologies are established. The two star topologies
    // should still be disconnected.
    let hub_a = star_a_nodes.first().unwrap();
    wait_until!(5, hub_a.peer_book.read().number_of_connected_peers() as usize == N - 1);
    let hub_b = star_b_nodes.first().unwrap();
    wait_until!(5, hub_b.peer_book.read().number_of_connected_peers() as usize == N - 1);

    // Setting up a list of nodes as we will consider them as a whole graph from this point
    // forwards.
    star_a_nodes.append(&mut star_b_nodes);
    let mut nodes = star_a_nodes;

    // Single node to connect to a subset of N and K.
    let bootnodes = vec![ba, bb];

    let solo_setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        min_peers: N as u16,
        bootnodes,
        ..Default::default()
    };
    let solo = test_node(solo_setup).await;
    nodes.push(solo);

    let density = || {
        let connections = total_connection_count(&nodes);
        network_density(nodes.len() as f64, connections as f64)
    };
    // wait_until!(10, density() >= 0.5);

    // let jh = start_rpc_server(nodes.clone()).await;
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn graph_test() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        min_peers: 3 as u16,
        max_peers: 7,
        ..Default::default()
    };
    let mut nodes = Arc::new(RwLock::new(test_nodes(N, setup).await));

    let jh = start_rpc_server(nodes.clone()).await;

    connect_nodes(&mut nodes.write(), Topology::Ring).await;
    start_nodes(&nodes.read()).await;

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let solo_setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 2,
        min_peers: 5 as u16,
        max_peers: 7,
        bootnodes: vec![nodes.read().first().unwrap().local_address().unwrap().to_string()],
        ..Default::default()
    };
    let solo = test_node(solo_setup).await;
    nodes.write().push(solo);
}

fn total_connection_count(nodes: &Vec<Node>) -> usize {
    let mut count = 0;

    for node in nodes {
        count += node.peer_book.read().number_of_connected_peers()
    }

    (count / 2).into()
}

// Topology metrics
//
// 1. node count
// 2. density
// 3. centrality measurements:
//
//      - degree centrality (covered by the number of connected peers)
//      - eigenvector centrality

fn network_density(n: f64, ac: f64) -> f64 {
    // Calculate the total number of possible connections given a node count.
    let pc = n * (n - 1.0) / 2.0;
    // Actual connections divided by the possbile connections gives the density.
    ac / pc
}

fn degree_centrality_delta(nodes: &Vec<Node>) -> u16 {
    let dc = nodes
        .iter()
        .map(|node| node.peer_book.read().number_of_connected_peers());
    let min = dc.clone().min().unwrap();
    let max = dc.max().unwrap();

    max - min
}

use serde::Serialize;
use std::{collections::HashSet, net::SocketAddr};

#[derive(Debug, Serialize, Eq, Hash, PartialEq, Copy, Clone)]
struct Vertex {
    id: SocketAddr,
    is_bootnode: bool,
}

#[derive(Debug, Serialize, Eq, Hash, PartialEq, Copy, Clone)]
struct Edge {
    source: SocketAddr,
    target: SocketAddr,
}

#[derive(Debug, Serialize, Clone)]
struct Graph {
    vertices: HashSet<Vertex>,
    edges: HashSet<Edge>,
}

#[derive(Debug, Serialize)]
struct GraphDiff {
    added_vertices: Vec<Vertex>,
    removed_vertices: Vec<Vertex>,
    added_edges: Vec<Edge>,
    removed_edges: Vec<Edge>,
}

impl Graph {
    fn new() -> Self {
        Self {
            vertices: HashSet::new(),
            edges: HashSet::new(),
        }
    }

    fn from(nodes: Vec<Node>) -> Self {
        let mut vertices = HashSet::new();
        let mut edges = HashSet::new();

        // Used only for dedup purposes.
        let mut connected_pairs = HashSet::new();

        for node in nodes {
            let own_addr = node.local_address().unwrap();
            vertices.insert(Vertex {
                id: own_addr,
                is_bootnode: node.environment.is_bootnode(),
            });

            for (addr, _peer_info) in node.peer_book.read().connected_peers() {
                if own_addr != *addr
                    && connected_pairs.insert((own_addr, *addr))
                    && connected_pairs.insert((*addr, own_addr))
                {
                    edges.insert(Edge {
                        source: own_addr,
                        target: *addr,
                    });
                }
            }
        }

        Self { vertices, edges }
    }

    //  Returns new state as well as an instance of Graph representing the Diff.
    fn update(&mut self, nodes: Vec<Node>) -> GraphDiff {
        // Self is the last sent state.
        let current_state = dbg!(Graph::from(nodes));

        // Compute the diffs.
        let removed_vertices: Vec<Vertex> = self.vertices.difference(&current_state.vertices).copied().collect();
        let removed_edges: Vec<Edge> = self.edges.difference(&current_state.edges).copied().collect();

        let added_vertices: Vec<Vertex> = current_state.vertices.difference(&self.vertices).copied().collect();
        let added_edges: Vec<Edge> = current_state.edges.difference(&self.edges).copied().collect();

        *self = current_state;

        GraphDiff {
            added_vertices,
            removed_vertices,
            added_edges,
            removed_edges,
        }
    }
}

use tokio::task::JoinHandle;
async fn start_rpc_server(nodes: Arc<RwLock<Vec<Node>>>) {
    use jsonrpc_http_server::{jsonrpc_core::IoHandler, AccessControlAllowOrigin, DomainsValidation, ServerBuilder};
    use serde_json::json;
    use tokio::task;

    let g = Arc::new(RwLock::new(Graph::new()));

    // Listener responds with the current graph every time an RPC call occures.
    let mut io = IoHandler::default();
    io.add_method("graph", move |_| {
        let diff = g.write().update(nodes.read().clone());
        Ok(json!(diff))
    });

    let server = ServerBuilder::new(io)
        .cors(DomainsValidation::AllowOnly(vec![AccessControlAllowOrigin::Null]))
        .start_http(&"127.0.0.1:3030".parse().unwrap())
        .expect("Unable to start RPC server");

    task::spawn(async {
        server.wait();
    });
}
