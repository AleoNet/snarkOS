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

// Network crawler:
// Start a crawler task (similar to the peers task) which updates state. Only one peer would be
// connected at a time to start and would be queried for peers. It would then select on peer at
// random to continue the crawl.
//
// Q: extend the network protocol to include statistics or node metadata?
// Q: when to perform centrality computation?

use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashSet},
    hash::{Hash, Hasher},
    net::SocketAddr,
    ops::Sub,
};

use nalgebra::{DMatrix, DVector, SymmetricEigen};

#[derive(Debug, Eq, Copy, Clone)]
struct Connection((SocketAddr, SocketAddr));

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        let (a, b) = self.0;
        let (c, d) = other.0;

        a == d && b == c || a == c && b == d
    }
}

impl Hash for Connection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (a, b) = self.0;

        match a.cmp(&b) {
            Ordering::Greater => {
                b.hash(state);
                a.hash(state);
            }

            _ => {
                a.hash(state);
                b.hash(state);
            }
        }
    }
}

/// Keeps track of crawled peers and their connections.
#[derive(Debug, Default)]
pub struct NetworkTopology {
    connections: HashSet<Connection>,
}

impl NetworkTopology {
    fn update(&mut self, source: SocketAddr, peers: Vec<SocketAddr>) {
        // Rules:
        //  - if a connecton exists already, do nothing.
        //  - if a connection is new, add it.
        //  - if an exisitng connection involving the source isn't in the peerlist, remove it.

        let new_connections: HashSet<Connection> = peers.into_iter().map(|peer| Connection((source, peer))).collect();

        // Find which connections need to be removed.
        //
        // With sets: a - b = removed connections (if and only if one of the two addrs is the
        // source), otherwise it's a connection which doesn't include the source and shouldn't be
        // removed.
        let connections_to_remove: HashSet<Connection> = self
            .connections
            .difference(&new_connections)
            .filter(|Connection((a, b))| a == &source || b == &source)
            .copied()
            .collect();

        // Only retain connections that aren't removed.
        self.connections
            .retain(|connection| !connections_to_remove.contains(&connection));

        // Insert new connections.
        self.connections.extend(new_connections.iter());
    }

    pub fn has_connections(&self) -> bool {
        self.connections.len() > 0
    }
}

/// Network topology measurements.
#[derive(Debug)]
pub struct NetworkMetrics {
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
    degree_centrality_delta: f64,
    /// Node centrality measurements mapped to each node's address.
    ///
    /// Includes degree centrality, eigenvector centrality (the relative importance of a node in
    /// the network) and Fiedler vector (describes a possible partitioning of the network).
    centrality: BTreeMap<SocketAddr, NodeCentrality>,
}

impl NetworkMetrics {
    /// Returns the network metrics for the state described by the connections list.
    pub fn new(topology: &NetworkTopology) -> Self {
        // Copy the connections as the data must not change throughout the metrics computation.
        let connections: HashSet<Connection> = topology.connections.iter().copied().collect();

        // Construct the list of nodes from the connections.
        let mut nodes: HashSet<SocketAddr> = HashSet::new();
        for Connection((a, b)) in connections.iter() {
            // Using a hashset guarantees uniqueness.
            nodes.insert(*a);
            nodes.insert(*b);
        }

        let node_count = nodes.len();
        let connection_count = connections.len();
        let density = calculate_density(node_count as f64, connection_count as f64);

        // Create an index of nodes to introduce some notion of order the rows and columns all matrices will follow.
        let index: BTreeMap<SocketAddr, usize> = nodes.iter().enumerate().map(|(i, &addr)| (addr, i)).collect();

        // Not stored on the struct but can be pretty inspected with `println!`.
        // The adjacency matrix can be built from the node index and the connections list.
        let adjacency_matrix = adjacency_matrix(&index, connections);
        // The degree matrix can be built from the adjacency matrix (row sum is connection count).
        let degree_matrix = degree_matrix(&index, &adjacency_matrix);
        // The laplacian matrix is degree matrix minus the adjacence matrix.
        let laplacian_matrix = degree_matrix.clone().sub(&adjacency_matrix);

        let degree_centrality = degree_centrality(&index, &degree_matrix.clone());
        let degree_centrality_delta = degree_centrality_delta(&degree_matrix);
        let eigenvector_centrality = eigenvector_centrality(&index, adjacency_matrix);
        let (algebraic_connectivity, fiedler_vector_indexed) = fiedler(&index, laplacian_matrix);

        // Create the `NodeCentrality` instances for each node.
        let centrality: BTreeMap<SocketAddr, NodeCentrality> = nodes
            .iter()
            .map(|&addr| {
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
fn adjacency_matrix(index: &BTreeMap<SocketAddr, usize>, connections: HashSet<Connection>) -> DMatrix<f64> {
    let n = index.len();
    let mut matrix = DMatrix::<f64>::zeros(n, n);

    // Compute the adjacency matrix. As our network is an undirected graph, the adjacency matrix is
    // symmetric.
    for Connection((a, b)) in connections {
        // Addresses must be present.
        // Get the indices for each address in the connection.
        let i = index.get(&a).unwrap();
        let j = index.get(&b).unwrap();

        // Since connections are unique both the upper and lower triangles must be writted (as the
        // graph is unidrected) for each connection.
        matrix[(*i, *j)] = 1.0;
        matrix[(*j, *i)] = 1.0;
    }

    matrix
}

/// Returns the difference between the highest and lowest degree centrality in the network.
///
/// Returns an `f64`, though the value should be a natural number.
fn degree_centrality_delta(degree_matrix: &DMatrix<f64>) -> f64 {
    let max = degree_matrix.max();
    let min = degree_matrix.min();

    max - min
}

/// Returns the degree centrality of a node.
///
/// This is defined as the connection count of the node.
fn degree_centrality(index: &BTreeMap<SocketAddr, usize>, degree_matrix: &DMatrix<f64>) -> BTreeMap<SocketAddr, u16> {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn connections_partial_eq() {
        let a = "12.34.56.78:9000".parse().unwrap();
        let b = "98.76.54.32:1000".parse().unwrap();

        assert_eq!(Connection((a, b)), Connection((b, a)));
        assert_eq!(Connection((a, b)), Connection((a, b)));
    }

    #[test]
    fn connections_update() {
        let a = "11.11.11.11:1000".parse().unwrap();
        let b = "22.22.22.22:2000".parse().unwrap();
        let c = "33.33.33.33:3000".parse().unwrap();

        let mut topology = NetworkTopology::default();

        // Insert two connections.
        topology.update(a, vec![b, c]);
        assert!(topology.connections.contains(&Connection((a, b))));
        assert!(topology.connections.contains(&Connection((a, c))));

        // Insert (a, b) connection reversed, make sure it doesn't change the list.
        topology.update(b, vec![a]);
        assert!(topology.connections.len() == 2);

        // Update c connections but don't include (c, a) == (a, c) and expect it to be removed.
        topology.update(c, vec![b]);
        assert!(!topology.connections.contains(&Connection((a, c))));
        assert!(topology.connections.contains(&Connection((c, b))));
    }

    #[test]
    fn connections_hash() {
        use std::collections::hash_map::DefaultHasher;

        let a = "11.11.11.11:1000".parse().unwrap();
        let b = "22.22.22.22:2000".parse().unwrap();

        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();

        let k1 = Connection((a, b));
        let k2 = Connection((b, a));

        k1.hash(&mut h1);
        k2.hash(&mut h2);

        // verify k1 == k2 => hash(k1) == hash(k2)
        assert_eq!(h1.finish(), h2.finish());
    }
}
