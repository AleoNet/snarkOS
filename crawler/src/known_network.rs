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
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
};
use time::OffsetDateTime;

use crate::connection::{nodes_from_connections, Connection};

// Purges connections that haven't been seen within this time (in hours).
const STALE_CONNECTION_CUTOFF_TIME_HRS: i64 = 4;

/// Keeps track of crawled peers and their connections.
#[derive(Debug, Default)]
pub struct KnownNetwork {
    // The nodes and their block height if known.
    nodes: RwLock<HashMap<SocketAddr, u32>>,
    // The connections map.
    connections: RwLock<HashSet<Connection>>,
}

impl KnownNetwork {
    // More convenient for testing.
    pub fn update_connections(&self, source: SocketAddr, peers: Vec<SocketAddr>) {
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

    use time::Duration;

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
