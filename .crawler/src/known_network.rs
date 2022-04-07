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

use parking_lot::RwLock;
use snarkos_environment::helpers::{NodeType, Status};
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
};
use time::{Duration, OffsetDateTime};

use crate::{
    connection::{nodes_from_connections, Connection},
    constants::*,
};

/// The current state of a crawled node.
#[derive(Debug, Clone)]
pub struct NodeState {
    pub node_type: NodeType,
    pub version: u32,
    pub height: u32,
    pub status: Status,
}

/// Node information collected while crawling.
#[derive(Debug, Clone, Default)]
pub struct NodeMeta {
    // The details of the node's state.
    pub state: Option<NodeState>,
    // The last interaction timestamp.
    pub timestamp: Option<OffsetDateTime>,
    // The number of lists of peers received from the node.
    received_peer_sets: u8,
    // The number of subsequent connection failures.
    connection_failures: u8,
    // The time it took to connect to the node.
    pub handshake_time: Option<Duration>,
}

impl NodeMeta {
    // Resets the node's values which determine whether the crawler should stay connected to it.
    // note: it should be called when a node is disconnected from after it's been crawled successfully
    fn reset_crawl_state(&mut self) {
        self.received_peer_sets = 0;
        self.connection_failures = 0;
        self.timestamp = Some(OffsetDateTime::now_utc());
    }

    // Returns `true` if the node should be connected to again.
    fn needs_refreshing(&self) -> bool {
        if let Some(timestamp) = self.timestamp {
            let crawl_interval = if self.state.is_some() {
                CRAWL_INTERVAL_MINS
            } else {
                // Delay further connection attempts to nodes that are hard to connect to.
                self.connection_failures as i64
            };

            (OffsetDateTime::now_utc() - timestamp).whole_minutes() > crawl_interval
        } else {
            // If there is no timestamp yet, this is the very first connection attempt.
            true
        }
    }
}

/// Keeps track of crawled peers and their connections.
// note: all the associated addresses are listening addresses.
#[derive(Debug, Default)]
pub struct KnownNetwork {
    // The information on known nodes; the keys of the map are their related listening addresses.
    nodes: RwLock<HashMap<SocketAddr, NodeMeta>>,
    // The map of known connections between nodes.
    connections: RwLock<HashSet<Connection>>,
}

impl KnownNetwork {
    /// Adds a node with the given address.
    pub fn add_node(&self, listening_addr: SocketAddr) {
        self.nodes.write().insert(listening_addr, NodeMeta::default());
    }

    // Updates the list of connections and registers new nodes based on them.
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
        let now = OffsetDateTime::now_utc();
        let connections_to_remove: Vec<Connection> = self
            .connections
            .read()
            .difference(&new_connections)
            .filter(|conn| {
                (conn.source == source || conn.target == source) && (now - conn.last_seen).whole_hours() > STALE_CONNECTION_CUTOFF_TIME_HRS
            })
            .copied()
            .collect();

        // Scope the write lock.
        {
            let mut connections_g = self.connections.write();

            // Remove stale connections, if there are any.
            for addr in connections_to_remove {
                connections_g.remove(&addr);
            }

            // Insert new connections, we use `replace` so the last seen timestamp is overwritten.
            for new_connection in new_connections.into_iter() {
                connections_g.replace(new_connection);
            }
        }

        // Obtain node addresses based on the list of known connections.
        let node_addrs_from_conns = nodes_from_connections(&self.connections());

        // Scope the write lock.
        {
            let mut nodes_g = self.nodes.write();

            // Create new node objects based on connection addresses.
            for addr in node_addrs_from_conns {
                nodes_g.entry(addr).or_default();
            }
        }
    }

    /// Updates the details of a node based on a Ping message received from it.
    pub fn received_ping(&self, source: SocketAddr, node_type: NodeType, version: u32, status: Status, height: u32) {
        let timestamp = OffsetDateTime::now_utc();

        let mut nodes = self.nodes.write();
        let mut meta = nodes.entry(source).or_default();

        meta.state = Some(NodeState {
            node_type,
            version,
            height,
            status,
        });
        meta.timestamp = Some(timestamp);
    }

    /// Updates the known connections based on a received list of a node's peers.
    pub fn received_peers(&self, source: SocketAddr, addrs: Vec<SocketAddr>) {
        let timestamp = OffsetDateTime::now_utc();

        self.update_connections(source, addrs);

        let mut nodes = self.nodes.write();
        let mut meta = nodes.entry(source).or_default();

        meta.received_peer_sets += 1;
        meta.timestamp = Some(timestamp);
    }

    /// Updates a node's details applicable as soon as a connection succeeds or fails.
    pub fn connected_to_node(&self, source: SocketAddr, connection_init_timestamp: OffsetDateTime, connection_succeeded: bool) {
        let mut nodes = self.nodes.write();
        let mut meta = nodes.entry(source).or_default();

        // Update the node interaction timestamp.
        meta.timestamp = Some(connection_init_timestamp);

        if connection_succeeded {
            // Reset the conn failure count when the connection succeeds.
            meta.connection_failures = 0;
            // Register the time it took to perform the handshake.
            meta.handshake_time = Some(OffsetDateTime::now_utc() - connection_init_timestamp);
        } else {
            meta.connection_failures += 1;
        }
    }

    /// Checks if the given address should be (re)connected to.
    pub fn should_be_connected_to(&self, addr: SocketAddr) -> bool {
        if let Some(meta) = self.nodes.read().get(&addr) {
            meta.needs_refreshing()
        } else {
            true
        }
    }

    /// Returns a list of addresses the crawler should connect to.
    pub fn addrs_to_connect(&self) -> HashSet<SocketAddr> {
        // Snapshot is safe to use as disconnected peers won't have their state updated at the
        // moment.
        self.nodes()
            .iter()
            .filter(|(_, meta)| meta.needs_refreshing())
            .map(|(&addr, _)| addr)
            .collect()
    }

    /// Returns a list of addresses the crawler should disconnect from.
    pub fn addrs_to_disconnect(&self) -> Vec<SocketAddr> {
        let mut peers = self.nodes.write();

        // Forget nodes that can't be connected to in case they are offline.
        peers.retain(|_, meta| meta.connection_failures <= MAX_CONNECTION_FAILURE_COUNT);

        let mut addrs = Vec::new();
        for (addr, meta) in peers.iter_mut() {
            // Disconnect from peers we have received the state and sufficient peers from.
            if meta.state.is_some() && meta.received_peer_sets >= DESIRED_PEER_SET_COUNT {
                meta.reset_crawl_state();
                addrs.push(*addr);
            }
        }

        addrs
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
    pub fn nodes(&self) -> HashMap<SocketAddr, NodeMeta> {
        self.nodes.read().clone()
    }

    /// Returns the number of all the known connections.
    pub fn num_connections(&self) -> usize {
        self.connections.read().len()
    }

    /// Returns the number of all the known nodes.
    pub fn num_nodes(&self) -> usize {
        self.nodes.read().len()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn connections_update() {
        let addr_a = "11.11.11.11:1000".parse().unwrap();
        let addr_b = "22.22.22.22:2000".parse().unwrap();
        let addr_c = "33.33.33.33:3000".parse().unwrap();
        let addr_d = "44.44.44.44:4000".parse().unwrap();
        let addr_e = "55.55.55.55:5000".parse().unwrap();

        let old_but_valid_timestamp = OffsetDateTime::now_utc() - Duration::hours(STALE_CONNECTION_CUTOFF_TIME_HRS - 1);
        let stale_timestamp = OffsetDateTime::now_utc() - Duration::hours(STALE_CONNECTION_CUTOFF_TIME_HRS + 1);

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

        let known_network = KnownNetwork {
            nodes: Default::default(),
            connections: RwLock::new(seeded_connections),
        };

        // Insert two connections.
        known_network.update_connections(addr_a, vec![addr_b, addr_c]);
        assert!(known_network.connections.read().contains(&Connection::new(addr_a, addr_b)));
        assert!(known_network.connections.read().contains(&Connection::new(addr_a, addr_c)));
        assert!(known_network.connections.read().contains(&Connection::new(addr_a, addr_d)));
        // Assert the stale connection was purged.
        assert!(!known_network.connections.read().contains(&Connection::new(addr_a, addr_e)));

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
}
