use parking_lot::RwLock;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    net::SocketAddr,
};
use time::OffsetDateTime;

use crate::connection::{nodes_from_connections, Connection};

// Purges connections that haven't been seen within this time (in hours).
const STALE_CONNECTION_CUTOFF_TIME_HRS: i64 = 4;

/// Keeps track of crawled peers and their connections.
#[derive(Debug)]
pub struct KnownNetwork {
    // The nodes and their block height if known.
    nodes: RwLock<HashMap<SocketAddr, u32>>,
    // The connections map.
    connections: RwLock<HashSet<Connection>>,
}

impl KnownNetwork {
    // More convenient for testing.
    fn update_connections(&self, source: SocketAddr, peers: Vec<SocketAddr>) {
        // Rules:
        //  - if a connecton exists already, do nothing.
        //  - if a connection is new, add it.
        //  - if an exisitng connection involving the source isn't in the peerlist, remove it if
        //  it's stale.

        let new_connections: HashSet<Connection> = peers.into_iter().map(|peer| Connection::new(source, peer)).collect();

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
                    && (OffsetDateTime::now_utc() - conn.last_seen).whole_hours() > STALE_CONNECTION_CUTOFF_TIME_HRS
            })
            .copied()
            .collect();

        // Scope the write lock.
        {
            let mut connections_g = self.connections.write();

            // Remove stale connections.
            connections_g.retain(|connection| !connections_to_remove.contains(connection));

            // Insert new connections, we use replace so the last seen timestamp is overwritten.
            for new_connection in new_connections.into_iter() {
                connections_g.replace(new_connection);
            }
        }

        // Scope the write lock.
        {
            let mut nodes_g = self.nodes.write();

            // Remove the nodes that no longer correspond to connections.
            let nodes_from_connections = nodes_from_connections(&self.connections());
            nodes_g.retain(|addr, _| nodes_from_connections.contains(addr));
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
}

#[cfg(test)]
mod test {
    use super::*;
}
