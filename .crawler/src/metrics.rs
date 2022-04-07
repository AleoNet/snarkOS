// Copyright (C) 2019-2022 Aleo Systems Inc.
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

#[cfg(not(feature = "postgres"))]
use std::{cmp, fmt};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    net::SocketAddr,
    ops::Sub,
};

use nalgebra::{DMatrix, DVector, SymmetricEigen};
#[cfg(not(feature = "postgres"))]
use snarkos_environment::helpers::{NodeType, Status};
#[cfg(not(feature = "postgres"))]
use time::Duration;

use crate::{connection::Connection, known_network::NodeMeta};

/// A summary of the state of the known nodes.
#[cfg(not(feature = "postgres"))]
#[derive(Clone)]
#[allow(dead_code)]
pub struct NetworkSummary {
    // The number of all known nodes.
    num_known_nodes: usize,
    // The number of all known connections.
    num_known_connections: usize,
    // The number of nodes that haven't provided their state yet.
    nodes_pending_state: usize,
    // The types of nodes and their respective counts.
    types: HashMap<NodeType, usize>,
    // The versions of nodes and their respective counts.
    versions: HashMap<u32, usize>,
    // The node statuses of nodes and their respective counts.
    statuses: HashMap<Status, usize>,
    // The heights of nodes and their respective counts.
    heights: HashMap<u32, usize>,
    // Corresponds to the same field in the NetworkMetrics.
    density: f64,
    // Corresponds to the same field in the NetworkMetrics.
    algebraic_connectivity: f64,
    // Corresponds to the same field in the NetworkMetrics.
    degree_centrality_delta: u16,
    // Average number of node connections.
    avg_degree_centrality: u16,
    // Average node height.
    avg_height: Option<u32>,
    // The average handshake time in the network.
    avg_handshake_time_ms: Option<i64>,
}

#[cfg(not(feature = "postgres"))]
impl fmt::Display for NetworkSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn print_breakdown<T: fmt::Display>(f: &mut fmt::Formatter<'_>, counts: &HashMap<T, usize>, limit: usize) -> fmt::Result {
            let mut vec: Vec<(&T, &usize)> = counts.iter().collect();
            vec.sort_by_key(|(_, count)| cmp::Reverse(*count));

            let limit = if limit == 0 { vec.len() } else { limit };
            for (i, (item, count)) in vec.iter().enumerate().take(limit) {
                write!(f, "{}: {}{}", item, count, if i < limit - 1 { ", " } else { "\n" })?;
            }

            Ok(())
        }

        writeln!(f, "\nNetwork summary:")?;
        writeln!(
            f,
            "there are {} nodes with {} connections between them",
            self.num_known_nodes, self.num_known_connections
        )?;
        if self.nodes_pending_state > 0 {
            writeln!(f, "{} node(s) have not been successfully crawled yet", self.nodes_pending_state)?;
        }

        writeln!(f, "\nBreakdown:")?;
        write!(f, "by node type: ")?;
        print_breakdown(f, &self.types, 0)?;
        write!(f, "by protocol version: ")?;
        print_breakdown(f, &self.versions, 0)?;
        write!(f, "by current state: ")?;
        print_breakdown(f, &self.statuses, 0)?;

        writeln!(f, "\nNetwork metrics:")?;
        writeln!(f, "density: {:.4}", self.density)?;
        writeln!(f, "algebraic connectivity: {:.4}", self.algebraic_connectivity)?;
        writeln!(f, "degree centrality delta: {}", self.degree_centrality_delta)?;
        writeln!(f, "average number of node connections: {}", self.avg_degree_centrality)?;
        if let Some(ms) = self.avg_handshake_time_ms {
            writeln!(f, "average handshake time: {}ms", ms)?;
        }

        writeln!(f, "\nBlockchain-related details:")?;
        if let Some(h) = self.heights.keys().max() {
            writeln!(f, "maximum found height: {}", h)?;
        }
        write!(f, "5 most common heights: ")?;
        print_breakdown(f, &self.heights, 5)?;
        if let Some(h) = self.avg_height {
            writeln!(f, "average height: {}", h)?;
        }

        Ok(())
    }
}

/// Network topology measurements.
#[derive(Debug)]
pub struct NetworkMetrics {
    /// The total node count of the network.
    pub node_count: usize,
    /// The total connection count for the network.
    pub connection_count: usize,
    /// The network density.
    ///
    /// This is defined as actual connections divided by the total number of possible connections.
    pub density: f64,
    /// The algebraic connectivity of the network.
    ///
    /// This is the value of the Fiedler eigenvalue, the second-smallest eigenvalue of the network's
    /// Laplacian matrix.
    pub algebraic_connectivity: f64,
    /// The difference between the node with the largest connection count and the node with the
    /// lowest.
    pub degree_centrality_delta: u16,
    /// Per-node metrics.
    pub per_node: Vec<(SocketAddr, NodeMeta, NodeCentrality)>,
}

impl NetworkMetrics {
    /// Returns the network metrics for the state described by the connections list.
    pub fn new(connections: HashSet<Connection>, nodes: HashMap<SocketAddr, NodeMeta>) -> Option<Self> {
        // Don't compute the metrics for an empty set of connections.
        if connections.is_empty() {
            // Returns all metrics set to `0`.
            return None;
        }

        let node_count = nodes.len();
        let connection_count = connections.len();
        let density = calculate_density(node_count as f64, connection_count as f64);

        // Create an index of nodes to introduce some notion of order the rows and columns all matrices will follow.
        let index: BTreeMap<SocketAddr, usize> = nodes.keys().enumerate().map(|(i, addr)| (*addr, i)).collect();

        // Not stored on the struct but can be pretty inspected with `println!`.
        // The adjacency matrix can be built from the node index and the connections list.
        let adjacency_matrix = adjacency_matrix(&index, &connections);
        // The degree matrix can be built from the adjacency matrix (row sum is connection count).
        let degree_matrix = degree_matrix(&index, &adjacency_matrix);
        // The laplacian matrix is degree matrix minus the adjacence matrix.
        let laplacian_matrix = degree_matrix.clone().sub(&adjacency_matrix);

        let degree_centrality = degree_centrality(&index, &degree_matrix);
        let degree_centrality_delta = degree_centrality_delta(&degree_matrix);
        let eigenvector_centrality = eigenvector_centrality(&index, adjacency_matrix);
        let (algebraic_connectivity, fiedler_vector_indexed) = fiedler(&index, laplacian_matrix);

        // Create the `NodeCentrality` instances for each node.
        let per_node = nodes
            .into_iter()
            .map(|(addr, meta)| {
                // Must contain values for this node since it was constructed using same set of
                // nodes.
                let dc = degree_centrality.get(&addr).unwrap();
                let ec = eigenvector_centrality.get(&addr).unwrap();
                let fv = fiedler_vector_indexed.get(&addr).unwrap();
                let nc = NodeCentrality::new(*dc, *ec, *fv);

                (addr, meta, nc)
            })
            .collect();

        let metrics = Self {
            node_count,
            connection_count,
            density,
            algebraic_connectivity,
            degree_centrality_delta,
            per_node,
        };

        Some(metrics)
    }

    /// Returns a state summary for the known nodes.
    #[cfg(not(feature = "postgres"))]
    pub fn summary(&self) -> NetworkSummary {
        let mut versions = HashMap::with_capacity(self.node_count);
        let mut statuses = HashMap::with_capacity(self.node_count);
        let mut types = HashMap::with_capacity(self.node_count);
        let mut heights = HashMap::with_capacity(self.node_count);

        let mut handshake_times = Vec::with_capacity(self.node_count);
        let mut degree_centralities = Vec::with_capacity(self.node_count);
        let mut nodes_pending_state: usize = 0;

        for (_, meta, centrality) in &self.per_node {
            if let Some(ref state) = meta.state {
                versions.entry(state.version).and_modify(|count| *count += 1).or_insert(1);
                statuses.entry(state.status).and_modify(|count| *count += 1).or_insert(1);
                types.entry(state.node_type).and_modify(|count| *count += 1).or_insert(1);
                heights.entry(state.height).and_modify(|count| *count += 1).or_insert(1);
            } else {
                nodes_pending_state += 1;
            }
            degree_centralities.push(centrality.degree_centrality);
            if let Some(time) = meta.handshake_time {
                handshake_times.push(time);
            }
        }

        let avg_degree_centrality = degree_centralities.iter().map(|v| *v as u64).sum::<u64>() / degree_centralities.len() as u64;

        let avg_height = if !heights.is_empty() {
            let (mut sum_heights, mut sum_counts) = (0u64, 0u64);
            for (height, count) in &heights {
                sum_heights += *height as u64 * *count as u64;
                sum_counts += *count as u64;
            }
            let avg = sum_heights / sum_counts;
            Some(avg as u32)
        } else {
            None
        };

        let avg_handshake_time_ms = if !handshake_times.is_empty() {
            let avg = handshake_times.iter().sum::<Duration>().whole_milliseconds() as i64 / handshake_times.len() as i64;
            Some(avg)
        } else {
            None
        };

        NetworkSummary {
            num_known_nodes: self.node_count,
            num_known_connections: self.connection_count,
            nodes_pending_state,
            versions,
            heights,
            statuses,
            types,
            density: self.density,
            algebraic_connectivity: self.algebraic_connectivity,
            degree_centrality_delta: self.degree_centrality_delta,
            avg_height,
            avg_degree_centrality: avg_degree_centrality as u16,
            avg_handshake_time_ms,
        }
    }
}

/// Centrality measurements of a node.
///
/// These include degree centrality, eigenvector centrality (the relative importance of a node
/// in the network) and Fiedler vector (describes a possible partitioning of the network).
#[derive(Debug)]
pub struct NodeCentrality {
    /// Connection count of the node.
    pub degree_centrality: u16,
    /// A measure of the relative importance of the node in the network.
    ///
    /// Summing the values of each node adds up to the number of nodes in the network. This was
    /// done to allow comparison between different network topologies irrespective of node count.
    pub eigenvector_centrality: f64,
    /// This value is extracted from the Fiedler eigenvector corresponding to the second smallest
    /// eigenvalue of the Laplacian matrix of the network.
    ///
    /// The network can be partitioned on the basis of these values (positive, negative and when
    /// relevant close to zero).
    pub fiedler_value: f64,
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

pub fn calculate_density(n: f64, ac: f64) -> f64 {
    // Calculate the total number of possible connections given a node count.
    let pc = n * (n - 1.0) / 2.0;
    // Actual connections divided by the possbile connections gives the density.
    ac / pc
}

/// Returns the degree matrix for the network with values ordered by the index.
fn degree_matrix(index: &BTreeMap<SocketAddr, usize>, adjacency_matrix: &DMatrix<f64>) -> DMatrix<f64> {
    let n = index.len();
    let mut matrix = DMatrix::<f64>::zeros(n, n);

    for (i, row) in adjacency_matrix.row_iter().enumerate() {
        // Set the diagonal to be the sum of connections in that row. The index isn't necessary
        // here since the rows are visited in order and the adjacency matrix is ordered after the
        // index.
        matrix[(i, i)] = row.sum()
    }

    matrix
}

/// Returns the adjacency matrix for the network with values ordered by the index.
fn adjacency_matrix(index: &BTreeMap<SocketAddr, usize>, connections: &HashSet<Connection>) -> DMatrix<f64> {
    let n = index.len();
    let mut matrix = DMatrix::<f64>::zeros(n, n);

    // Compute the adjacency matrix. As our network is an undirected graph, the adjacency matrix is
    // symmetric.
    for connection in connections {
        // Addresses must be present.
        // Get the indices for each address in the connection.
        let i = index.get(&connection.source).unwrap();
        let j = index.get(&connection.target).unwrap();

        // Since connections are unique both the upper and lower triangles must be writted (as the
        // graph is unidrected) for each connection.
        matrix[(*i, *j)] = 1.0;
        matrix[(*j, *i)] = 1.0;
    }

    matrix
}

/// Returns the difference between the highest and lowest degree centrality in the network.
fn degree_centrality_delta(degree_matrix: &DMatrix<f64>) -> u16 {
    let max = degree_matrix.max();
    let min = degree_matrix.min();

    (max - min) as u16
}

/// Returns the degree centrality of a node.
///
/// This is defined as the connection count of the node.
fn degree_centrality(index: &BTreeMap<SocketAddr, usize>, degree_matrix: &DMatrix<f64>) -> HashMap<SocketAddr, u16> {
    let diag = degree_matrix.diagonal();
    index.keys().zip(diag.iter()).map(|(addr, dc)| (*addr, *dc as u16)).collect()
}

/// Returns the eigenvalue centrality of each node in the network.
fn eigenvector_centrality(index: &BTreeMap<SocketAddr, usize>, adjacency_matrix: DMatrix<f64>) -> HashMap<SocketAddr, f64> {
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
fn fiedler(index: &BTreeMap<SocketAddr, usize>, laplacian_matrix: DMatrix<f64>) -> (f64, HashMap<SocketAddr, f64>) {
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
