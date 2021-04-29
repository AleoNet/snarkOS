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
use snarkos_storage::LedgerStorage;
use snarkos_testing::{
    network::{
        test_config,
        test_node,
        topology::{connect_nodes, Topology},
        TestSetup,
    },
    wait_until,
};

use std::{collections::BTreeMap, net::SocketAddr, ops::Sub};

use nalgebra::{DMatrix, DVector, SymmetricEigen};

const N: usize = 25;
const MIN_PEERS: u16 = 5;
const MAX_PEERS: u16 = 30;

async fn test_nodes(n: usize, setup: TestSetup) -> Vec<Node<LedgerStorage>> {
    let mut nodes = Vec::with_capacity(n);

    for _ in 0..n {
        let environment = test_config(setup.clone());
        let node = Node::new(environment).await.unwrap();

        node.listen().await.unwrap();
        nodes.push(node);
    }

    nodes
}

async fn start_nodes(nodes: &[Node<LedgerStorage>]) {
    for node in nodes {
        // Nodes are started with a slight delay to avoid having peering intervals in phase (this
        // is the hypothetical worst case scenario).
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        node.start_services().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn spawn_nodes_in_a_line() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Line).await;
    start_nodes(&nodes).await;

    // First and Last nodes should have 1 connected peer.
    wait_until!(5, nodes.first().unwrap().peer_book.number_of_connected_peers() == 1);
    wait_until!(5, nodes.last().unwrap().peer_book.number_of_connected_peers() == 1);

    // All other nodes should have two.
    for node in nodes.iter().take(nodes.len() - 1).skip(1) {
        wait_until!(5, node.peer_book.number_of_connected_peers() == 2);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn spawn_nodes_in_a_ring() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Ring).await;
    start_nodes(&nodes).await;

    for node in &nodes {
        wait_until!(5, node.peer_book.number_of_connected_peers() == 2);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn spawn_nodes_in_a_star() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Star).await;
    start_nodes(&nodes).await;

    let hub = nodes.first().unwrap();
    wait_until!(10, hub.peer_book.number_of_connected_peers() as usize == N - 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn spawn_nodes_in_a_mesh() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 5,
        min_peers: MIN_PEERS,
        max_peers: MAX_PEERS,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Mesh).await;
    start_nodes(&nodes).await;

    // Set the sleep interval to 200ms to avoid locking issues.
    // Density measurement here is proportional to the min peers: if every node in the network
    // only connected to the min node count, the total number of connections would be roughly 10
    // percent of the total possible. With 50 nodes and min at 5 connections each this works out to
    // be 125/1225 ≈ 0.1.
    wait_until!(15, network_density(&nodes) >= 0.1, 200);

    // Make sure the node with the largest degree centrality and smallest degree centrality don't
    // have a delta greater than the max-min peer interval allows for. This check also provides
    // some insight into whether the network is meshed in a homogeneous manner.
    wait_until!(15, degree_centrality_delta(&nodes) <= MAX_PEERS - MIN_PEERS, 200);
}

#[tokio::test(flavor = "multi_thread")]
async fn line_converges_to_mesh() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        min_peers: MIN_PEERS,
        max_peers: MAX_PEERS,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Line).await;
    start_nodes(&nodes).await;

    wait_until!(10, network_density(&nodes) >= 0.1, 200);
    wait_until!(10, degree_centrality_delta(&nodes) <= MAX_PEERS - MIN_PEERS, 200);
}

#[tokio::test(flavor = "multi_thread")]
async fn ring_converges_to_mesh() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        min_peers: MIN_PEERS,
        max_peers: MAX_PEERS,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Ring).await;
    start_nodes(&nodes).await;

    wait_until!(10, network_density(&nodes) >= 0.1, 200);
    wait_until!(10, degree_centrality_delta(&nodes) <= MAX_PEERS - MIN_PEERS, 200);
}

#[tokio::test(flavor = "multi_thread")]
async fn star_converges_to_mesh() {
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        min_peers: MIN_PEERS,
        max_peers: MAX_PEERS,
        ..Default::default()
    };
    let mut nodes = test_nodes(N, setup).await;
    connect_nodes(&mut nodes, Topology::Star).await;
    start_nodes(&nodes).await;

    wait_until!(15, network_density(&nodes) >= 0.1, 200);
    wait_until!(15, degree_centrality_delta(&nodes) <= MAX_PEERS - MIN_PEERS, 200);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn binary_star_contact() {
    // Two initally separate star topologies subsequently connected by a single node connecting to
    // their bootnodes.

    // Setup the bootnodes for each star topology.
    let bootnode_setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        is_bootnode: true,
        ..Default::default()
    };
    let environment_a = test_config(bootnode_setup.clone());
    let environment_b = test_config(bootnode_setup.clone());
    let bootnode_a = Node::new(environment_a).await.unwrap();
    let bootnode_b = Node::new(environment_b).await.unwrap();

    bootnode_a.listen().await.unwrap();
    bootnode_b.listen().await.unwrap();

    let ba = bootnode_a.local_address().unwrap().to_string();
    let bb = bootnode_b.local_address().unwrap().to_string();

    // Create the nodes to be used as the leafs in the stars.
    let setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        min_peers: MIN_PEERS,
        max_peers: MAX_PEERS,
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

    // Start the services. The two meshes should still be disconnected.
    start_nodes(&star_a_nodes).await;
    start_nodes(&star_b_nodes).await;

    // Setting up a list of nodes as we will consider them as a whole graph from this point
    // forwards.
    star_a_nodes.append(&mut star_b_nodes);
    let mut nodes = star_a_nodes;

    // Single node to connect to a subset of N and K.
    let bootnodes = vec![ba, bb];

    let solo_setup = TestSetup {
        consensus_setup: None,
        peer_sync_interval: 1,
        min_peers: MIN_PEERS,
        max_peers: MAX_PEERS,
        bootnodes,
        ..Default::default()
    };
    let solo = test_node(solo_setup).await;
    nodes.push(solo);

    wait_until!(10, network_density(&nodes) >= 0.05);

    // Computing the metrics for this ignored case, interesting to inspect, especially Fiedler
    // partitioning as we have a graph with two clusters both centered around the bootnodes.
    let metrics = NetworkMetrics::new(&nodes);
    assert_eq!(metrics.node_count, 51);
}

/// Network topology measurements.
#[derive(Debug)]
struct NetworkMetrics {
    /// The total node count of the network.
    node_count: usize,
    /// The total connection count for the network.
    connection_count: usize,
    /// The network density.
    ///
    /// This is defined as actual connections divided by the total number of possible connections.
    density: f64,
    /// The algebraic connectivity of the network.
    ///
    /// This is the value of the Fiedler eigenvalue, the second-smallest eigenvalue of the network's
    /// Laplacian matrix.
    algebraic_connectivity: f64,
    /// The difference between the node with the largest connection count and the node with the
    /// lowest.
    degree_centrality_delta: u16,
    /// Node centrality measurements mapped to each node's address.
    ///
    /// Includes degree centrality, eigenvector centrality (the relative importance of a node in
    /// the network) and Fiedler vector (describes a possible partitioning of the network).
    centrality: BTreeMap<SocketAddr, NodeCentrality>,
}

impl NetworkMetrics {
    /// Returns the network metrics for the state described by the node list.
    fn new(nodes: &[Node<LedgerStorage>]) -> Self {
        let node_count = nodes.len();
        let connection_count = total_connection_count(nodes);
        let density = network_density(&nodes);

        // Create an index of nodes to introduce some notion of order the rows and columns all matrices will follow.
        let index: BTreeMap<SocketAddr, usize> = nodes
            .iter()
            .map(|node| node.local_address().unwrap())
            .enumerate()
            .map(|(i, addr)| (addr, i))
            .collect();

        // Not stored on the struct but can be pretty inspected with `println!`.
        let degree_matrix = degree_matrix(&index, &nodes);
        let adjacency_matrix = adjacency_matrix(&index, &nodes);
        let laplacian_matrix = degree_matrix.clone().sub(adjacency_matrix.clone());

        let degree_centrality = degree_centrality(&index, degree_matrix);
        let degree_centrality_delta = degree_centrality_delta(&nodes);
        let eigenvector_centrality = eigenvector_centrality(&index, adjacency_matrix);
        let (algebraic_connectivity, fiedler_vector_indexed) = fiedler(&index, laplacian_matrix);

        // Create the `NodeCentrality` instances for each node.
        let centrality: BTreeMap<SocketAddr, NodeCentrality> = nodes
            .iter()
            .map(|node| {
                let addr = node.local_address().unwrap();
                // Must contain values for this node since it was constructed using same set of
                // nodes.
                let dc = degree_centrality.get(&addr).unwrap();
                let ec = eigenvector_centrality.get(&addr).unwrap();
                let fv = fiedler_vector_indexed.get(&addr).unwrap();
                let nc = NodeCentrality::new(*dc, *ec, *fv);

                (addr, nc)
            })
            .collect();

        Self {
            node_count,
            connection_count,
            density,
            algebraic_connectivity,
            degree_centrality_delta,
            centrality,
        }
    }
}

/// Centrality measurements of a node.
#[derive(Debug)]
struct NodeCentrality {
    /// Connection count of the node.
    degree_centrality: u16,
    /// A measure of the relative importance of the node in the network.
    ///
    /// Summing the values of each node adds up to the number of nodes in the network. This was
    /// done to allow comparison between different network topologies irrespective of node count.
    eigenvector_centrality: f64,
    /// This value is extracted from the Fiedler eigenvector corresponding to the second smallest
    /// eigenvalue of the Laplacian matrix of the network.
    ///
    /// The network can be partitioned on the basis of these values (positive, negative and when
    /// relevant close to zero).
    fiedler_value: f64,
}

impl NodeCentrality {
    fn new(degree_centrality: u16, eigenvector_centrality: f64, fiedler_value: f64) -> Self {
        Self {
            degree_centrality,
            eigenvector_centrality,
            fiedler_value,
        }
    }
}

/// Returns the total connection count of the network.
fn total_connection_count(nodes: &[Node<LedgerStorage>]) -> usize {
    let mut count = 0;

    for node in nodes {
        count += node.peer_book.number_of_connected_peers()
    }

    (count / 2).into()
}

/// Returns the network density.
fn network_density(nodes: &[Node<LedgerStorage>]) -> f64 {
    let connections = total_connection_count(nodes);
    calculate_density(nodes.len() as f64, connections as f64)
}

fn calculate_density(n: f64, ac: f64) -> f64 {
    // Calculate the total number of possible connections given a node count.
    let pc = n * (n - 1.0) / 2.0;
    // Actual connections divided by the possbile connections gives the density.
    ac / pc
}

/// Returns the degree matrix for the network with values ordered by the index.
fn degree_matrix(index: &BTreeMap<SocketAddr, usize>, nodes: &[Node<LedgerStorage>]) -> DMatrix<f64> {
    let n = nodes.len();
    let mut matrix = DMatrix::<f64>::zeros(n, n);

    for node in nodes {
        let n = node.peer_book.number_of_connected_peers();
        // Address must be present.
        // Get the index for the and set the number of connected peers. The degree matrix is
        // diagonal.
        let node_n = index.get(&node.local_address().unwrap()).unwrap();
        matrix[(*node_n, *node_n)] = n as f64;
    }

    matrix
}

/// Returns the adjacency matrix for the network with values ordered by the index.
fn adjacency_matrix(index: &BTreeMap<SocketAddr, usize>, nodes: &[Node<LedgerStorage>]) -> DMatrix<f64> {
    let n = nodes.len();
    let mut matrix = DMatrix::<f64>::zeros(n, n);

    // Compute the adjacency matrix. As our network is an undirected graph, the adjacency matrix is
    // symmetric.
    for node in nodes {
        node.peer_book.connected_peers().keys().for_each(|addr| {
            // Addresses must be present.
            // Get the indices for each node, progressing row by row to construct the matrix.
            let node_m = index.get(&node.local_address().unwrap()).unwrap();
            let peer_n = index.get(&addr).unwrap();
            matrix[(*node_m, *peer_n)] = 1.0;
        });
    }

    matrix
}

/// Returns the difference between the highest and lowest degree centrality in the network.
// This could use the degree matrix, though as this is used extensively in tests and checked
// repeatedly until it reaches a certain value, we want to keep its calculation decoupled from the
// `NetworkMetrics`.
fn degree_centrality_delta(nodes: &[Node<LedgerStorage>]) -> u16 {
    let dc = nodes.iter().map(|node| node.peer_book.number_of_connected_peers());
    let min = dc.clone().min().unwrap();
    let max = dc.max().unwrap();

    max - min
}

/// Returns the degree centrality of a node.
///
/// This is defined as the connection count of the node.
fn degree_centrality(index: &BTreeMap<SocketAddr, usize>, degree_matrix: DMatrix<f64>) -> BTreeMap<SocketAddr, u16> {
    let diag = degree_matrix.diagonal();
    index
        .keys()
        .zip(diag.iter())
        .map(|(addr, dc)| (*addr, *dc as u16))
        .collect()
}

/// Returns the eigenvalue centrality of each node in the network.
fn eigenvector_centrality(
    index: &BTreeMap<SocketAddr, usize>,
    adjacency_matrix: DMatrix<f64>,
) -> BTreeMap<SocketAddr, f64> {
    // Compute the eigenvectors and corresponding eigenvalues and sort in descending order.
    let ascending = false;
    let eigenvalue_vector_pairs = sorted_eigenvalue_vector_pairs(adjacency_matrix, ascending);
    let (_highest_eigenvalue, highest_eigenvector) = &eigenvalue_vector_pairs[0];

    // The eigenvector is a relative score of node importance (normalised by the norm), to obtain an absolute score for each
    // node, we normalise so that the sum of the components are equal to 1.
    let sum = highest_eigenvector.sum() / index.len() as f64;
    let normalised = highest_eigenvector.unscale(sum);

    // Map addresses to their eigenvalue centrality.
    index
        .keys()
        .zip(normalised.column(0).iter())
        .map(|(addr, ec)| (*addr, *ec))
        .collect()
}

/// Returns the Fiedler values for each node in the network.
fn fiedler(index: &BTreeMap<SocketAddr, usize>, laplacian_matrix: DMatrix<f64>) -> (f64, BTreeMap<SocketAddr, f64>) {
    // Compute the eigenvectors and corresponding eigenvalues and sort in ascending order.
    let ascending = true;
    let pairs = sorted_eigenvalue_vector_pairs(laplacian_matrix, ascending);

    // Second-smallest eigenvalue is the Fiedler value (algebraic connectivity), the associated
    // eigenvector is the Fiedler vector.
    let (algebraic_connectivity, fiedler_vector) = &pairs[1];

    // Map addresses to their Fiedler values.
    let fiedler_values_indexed = index
        .keys()
        .zip(fiedler_vector.column(0).iter())
        .map(|(addr, fiedler_value)| (*addr, *fiedler_value))
        .collect();

    (*algebraic_connectivity, fiedler_values_indexed)
}

/// Computes the eigenvalues and corresponding eigenvalues from the supplied symmetric matrix.
fn sorted_eigenvalue_vector_pairs(matrix: DMatrix<f64>, ascending: bool) -> Vec<(f64, DVector<f64>)> {
    // Compute eigenvalues and eigenvectors.
    let eigen = SymmetricEigen::new(matrix);

    // Map eigenvalues to their eigenvectors.
    let mut pairs: Vec<(f64, DVector<f64>)> = eigen
        .eigenvalues
        .iter()
        .zip(eigen.eigenvectors.column_iter())
        .map(|(value, vector)| (*value, vector.clone_owned()))
        .collect();

    // Sort eigenvalue-vector pairs in descending order.
    pairs.sort_unstable_by(|(a, _), (b, _)| {
        if ascending {
            a.partial_cmp(b).unwrap()
        } else {
            b.partial_cmp(a).unwrap()
        }
    });

    pairs
}
