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

use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap, HashSet},
    hash::{Hash, Hasher},
    net::SocketAddr,
    ops::Sub,
};

use chrono::{DateTime, Utc};
use nalgebra::{DMatrix, DVector, SymmetricEigen};
use parking_lot::RwLock;
use tokio::sync::{
    mpsc,
    mpsc::{Receiver, Sender},
    Mutex,
};

// Purges connections that haven't been seen within this time (in hours).
const STALE_CONNECTION_CUTOFF_TIME_HRS: i64 = 4;

/// A connection between two peers.
///
/// Implements `partialEq` and `Hash` manually so that the `source`-`target` order has no impact on equality
/// (since connections are directionless). The timestamp is also not included in the comparison.
#[derive(Debug, Eq, Copy, Clone)]
pub struct Connection {
    /// One side of the connection.
    pub source: SocketAddr,
    /// The other side of the connection.
    pub target: SocketAddr,
    /// The last time this peer was seen by the crawler (used determine which connections are
    /// likely stale).
    last_seen: DateTime<Utc>,
}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        let (a, b) = (self.source, self.target);
        let (c, d) = (other.source, other.target);

        a == d && b == c || a == c && b == d
    }
}

impl Hash for Connection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (a, b) = (self.source, self.target);

        // This ensures the hash is the same for (a, b) as it is for (b, a).
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

impl Connection {
    fn new(source: SocketAddr, target: SocketAddr) -> Self {
        Connection {
            source,
            target,
            last_seen: Utc::now(),
        }
    }
}

pub enum KnownNetworkMessage {
    Peers((SocketAddr, Vec<SocketAddr>)),
    Height((SocketAddr, u32)),
}

/// Keeps track of crawled peers and their connections.
#[derive(Debug)]
pub struct KnownNetwork {
    pub sender: Sender<KnownNetworkMessage>,
    receiver: Mutex<Receiver<KnownNetworkMessage>>,

    // The nodes and their block height if known.
    nodes: RwLock<HashMap<SocketAddr, u32>>,
    connections: RwLock<HashSet<Connection>>,
}

impl Default for KnownNetwork {
    fn default() -> Self {
        // Buffer size of 1000 messages seems reasonable to begin with.
        let (tx, rx) = mpsc::channel(1000);

        Self {
            sender: tx,
            receiver: Mutex::new(rx),
            nodes: Default::default(),
            connections: Default::default(),
        }
    }
}

impl KnownNetwork {
    /// Updates the crawled connection set.
    pub async fn update(&self) {
        if let Some(message) = self.receiver.lock().await.recv().await {
            match message {
                KnownNetworkMessage::Peers((source, peers)) => self.update_connections(source, peers),
                KnownNetworkMessage::Height((source, height)) => self.update_height(source, height),
            }
        }
    }

    // More convenient for testing.
    fn update_connections(&self, source: SocketAddr, peers: Vec<SocketAddr>) {
        // Rules:
        //  - if a connecton exists already, do nothing.
        //  - if a connection is new, add it.
        //  - if an exisitng connection involving the source isn't in the peerlist, remove it if
        //  it's stale.

        let new_connections: HashSet<Connection> =
            peers.into_iter().map(|peer| Connection::new(source, peer)).collect();

        // Find which connections need to be removed.
        //
        // With sets: a - b = removed connections (if and only if one of the two addrs is the
        // source), otherwise it's a connection which doesn't include the source and shouldn't be
        // removed. We also keep connections seen within the last few hours as peerlists are capped
        // in size and omitted connections don't necessarily mean they don't exist anymore.
        let connections_to_remove: HashSet<Connection> = self
            .connections
            .read()
            .difference(&new_connections)
            .filter(|conn| {
                (conn.source == source || conn.target == source)
                    && (Utc::now() - conn.last_seen).num_hours() > STALE_CONNECTION_CUTOFF_TIME_HRS
            })
            .copied()
            .collect();

        // Only retain connections that aren't removed.
        self.connections
            .write()
            .retain(|connection| !connections_to_remove.contains(connection));

        // Scope the write lock.
        {
            let mut connections_g = self.connections.write();

            // Insert new connections, we use replace so the last seen timestamp is overwritten.
            for new_connection in new_connections.into_iter() {
                connections_g.replace(new_connection);
            }
        }

        // Scope the write lock.
        {
            let mut nodes_g = self.nodes.write();

            // Remove the nodes that no longer correspond to connections.
            let node_addrs: HashSet<SocketAddr> = nodes_g.iter().map(|(&addr, _)| addr).collect();
            let diff: HashSet<SocketAddr> = node_addrs.difference(&self.nodes_from_connections()).copied().collect();
            nodes_g.retain(|addr, _| !diff.contains(addr));
        }
    }

    /// Update the height stored for this particular node.
    pub fn update_height(&self, source: SocketAddr, height: u32) {
        self.nodes.write().insert(source, height);
    }

    /// Returns `true` if the known network contains any connections, `false` otherwise.
    pub fn has_connections(&self) -> bool {
        !self.connections.read().is_empty()
    }

    /// Returns a connection.
    pub fn get_connection(&self, source: SocketAddr, target: SocketAddr) -> Option<Connection> {
        self.connections.read().get(&Connection::new(source, target)).copied()
    }

    /// Returns a snapshot of all the connections.
    pub fn connections(&self) -> HashSet<Connection> {
        self.connections.read().clone()
    }

    /// Returns a snapshot of all the nodes.
    pub fn nodes(&self) -> HashMap<SocketAddr, u32> {
        self.nodes.read().clone()
    }

    /// Constructs a set of nodes contained from the connection set.
    pub fn nodes_from_connections(&self) -> HashSet<SocketAddr> {
        let mut nodes: HashSet<SocketAddr> = HashSet::new();
        for connection in self.connections().iter() {
            // Using a hashset guarantees uniqueness.
            nodes.insert(connection.source);
            nodes.insert(connection.target);
        }

        nodes
    }

    /// Returns a map of the potential forks.
    pub fn potential_forks(&self) -> HashMap<u32, Vec<SocketAddr>> {
        use itertools::Itertools;

        const HEIGHT_DELTA_TOLERANCE: u32 = 5;
        const MIN_CLUSTER_SIZE: usize = 3;

        let mut nodes: Vec<(SocketAddr, u32)> = self.nodes().into_iter().collect();
        nodes.sort_unstable_by_key(|&(_, height)| height);

        // Find the indexes at which the split the heights.
        let split_indexes: Vec<usize> = nodes
            .iter()
            .tuple_windows()
            .enumerate()
            .filter(|(_i, (a, b))| b.1 - a.1 >= HEIGHT_DELTA_TOLERANCE)
            .map(|(i, _)| i)
            .collect();

        // Create the clusters based on the indexes.
        let mut nodes_grouped = Vec::with_capacity(nodes.len());
        for i in split_indexes.iter().rev() {
            // The index needs to be offset by one.
            nodes_grouped.insert(0, nodes.split_off(*i + 1));
        }

        // Don't forget the first cluster left after the `split_off` operation.
        nodes_grouped.insert(0, nodes);

        // Remove the last cluster since it will contain the nodes even with the chain tip.
        nodes_grouped.pop();

        // Filter out any clusters smaller than three nodes, this minimises the false-positives
        // as it's reasonable to assume a fork would include more than two members.
        nodes_grouped.retain(|s| s.len() >= MIN_CLUSTER_SIZE);

        let mut potential_forks = HashMap::new();
        for cluster in nodes_grouped {
            // Safe since no clusters are of length `0`.
            let max_height = cluster.iter().map(|(_, height)| height).max().unwrap();
            let addrs = cluster.iter().map(|(addr, _)| addr).copied().collect();

            potential_forks.insert(*max_height, addrs);
        }

        potential_forks
    }
}

/// Network topology measurements.
#[derive(Debug, Default)]
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
    pub degree_centrality_delta: f64,
    /// Node centrality measurements mapped to each node's address.
    ///
    /// Includes degree centrality, eigenvector centrality (the relative importance of a node in
    /// the network) and Fiedler vector (describes a possible partitioning of the network).
    pub centrality: BTreeMap<SocketAddr, NodeCentrality>,
}

impl NetworkMetrics {
    /// Returns the network metrics for the state described by the connections list.
    pub fn new(connections: HashSet<Connection>) -> Self {
        // Don't compute the metrics for an empty set of connections.
        if connections.is_empty() {
            // Returns all metrics set to `0`.
            return Self::default();
        }

        // Construct the list of nodes from the connections.
        // FIXME: dedup?
        let mut nodes: HashSet<SocketAddr> = HashSet::new();
        for connection in connections.iter() {
            // Using a hashset guarantees uniqueness.
            nodes.insert(connection.source);
            nodes.insert(connection.target);
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

        let degree_centrality = degree_centrality(&index, &degree_matrix);
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
fn adjacency_matrix(index: &BTreeMap<SocketAddr, usize>, connections: HashSet<Connection>) -> DMatrix<f64> {
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
    use chrono::Duration;

    #[test]
    fn connections_partial_eq() {
        let a = "12.34.56.78:9000".parse().unwrap();
        let b = "98.76.54.32:1000".parse().unwrap();

        assert_eq!(Connection::new(a, b), Connection::new(b, a));
        assert_eq!(Connection::new(a, b), Connection::new(a, b));
    }

    #[test]
    fn connections_update() {
        let addr_a = "11.11.11.11:1000".parse().unwrap();
        let addr_b = "22.22.22.22:2000".parse().unwrap();
        let addr_c = "33.33.33.33:3000".parse().unwrap();
        let addr_d = "44.44.44.44:4000".parse().unwrap();
        let addr_e = "55.55.55.55:5000".parse().unwrap();

        let old_but_valid_timestamp = Utc::now() - Duration::hours(STALE_CONNECTION_CUTOFF_TIME_HRS - 1);
        let stale_timestamp = Utc::now() - Duration::hours(STALE_CONNECTION_CUTOFF_TIME_HRS + 1);

        // Seed the known network with the older connections.
        let old_but_valid_connection = Connection {
            source: addr_a,
            target: addr_d,
            last_seen: old_but_valid_timestamp,
        };

        let stale_connection = Connection {
            source: addr_a,
            target: addr_e,
            last_seen: stale_timestamp,
        };

        let mut seeded_connections = HashSet::new();
        seeded_connections.insert(old_but_valid_connection);
        seeded_connections.insert(stale_connection);

        let (tx, rx) = mpsc::channel(100);
        let known_network = KnownNetwork {
            sender: tx,
            receiver: Mutex::new(rx),
            nodes: RwLock::new(HashMap::new()),
            connections: RwLock::new(seeded_connections),
        };

        // Insert two connections.
        known_network.update_connections(addr_a, vec![addr_b, addr_c]);
        assert!(
            known_network
                .connections
                .read()
                .contains(&Connection::new(addr_a, addr_b))
        );
        assert!(
            known_network
                .connections
                .read()
                .contains(&Connection::new(addr_a, addr_c))
        );
        assert!(
            known_network
                .connections
                .read()
                .contains(&Connection::new(addr_a, addr_d))
        );
        // Assert the stale connection was purged.
        assert!(
            !known_network
                .connections
                .read()
                .contains(&Connection::new(addr_a, addr_e))
        );

        // Insert (a, b) connection reversed, make sure it doesn't change the list.
        known_network.update_connections(addr_b, vec![addr_a]);
        assert_eq!(known_network.connections.read().len(), 3);

        // Insert (a, d) again and make sure the timestamp was updated.
        known_network.update_connections(addr_a, vec![addr_d]);
        assert_ne!(
            old_but_valid_timestamp,
            known_network.get_connection(addr_a, addr_d).unwrap().last_seen
        );
    }

    #[test]
    fn connections_hash() {
        use std::collections::hash_map::DefaultHasher;

        let a = "11.11.11.11:1000".parse().unwrap();
        let b = "22.22.22.22:2000".parse().unwrap();

        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();

        let k1 = Connection::new(a, b);
        let k2 = Connection::new(b, a);

        k1.hash(&mut h1);
        k2.hash(&mut h2);

        // verify k1 == k2 => hash(k1) == hash(k2)
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn fork_detection() {
        let addr_a = "11.11.11.11:1000".parse().unwrap();
        let addr_b = "22.22.22.22:2000".parse().unwrap();
        let addr_c = "33.33.33.33:3000".parse().unwrap();
        let addr_d = "44.44.44.44:4000".parse().unwrap();
        let addr_e = "55.55.55.55:5000".parse().unwrap();
        let addr_f = "66.66.66.66:6000".parse().unwrap();

        let (tx, rx) = mpsc::channel(100);
        let known_network = KnownNetwork {
            sender: tx,
            receiver: Mutex::new(rx),
            nodes: RwLock::new(
                vec![
                    (addr_b, 24),
                    (addr_a, 1),
                    (addr_d, 26),
                    (addr_f, 50),
                    (addr_c, 25),
                    (addr_e, 50),
                ]
                .into_iter()
                .collect(),
            ),
            connections: RwLock::new(HashSet::new()),
        };

        let potential_forks = known_network.potential_forks();
        let expected_potential_forks: HashMap<u32, Vec<SocketAddr>> =
            vec![(26, vec![addr_b, addr_c, addr_d])].into_iter().collect();

        assert_eq!(potential_forks, expected_potential_forks);
    }
}
