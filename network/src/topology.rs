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
    collections::HashSet,
    hash::{Hash, Hasher},
    net::SocketAddr,
};

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
#[derive(Default)]
struct NetworkTopology {
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
}

// impl<S: Storage> Node<S> {
//     pub(crate) fn crawl_peers(&self, crawl_node: SocketAddr) {
//         // 1. Connect, handshake and request peers.
//         // 2. Store links in Crawler.
//
//         // Establish a connection with the selected peer.
//         tokio::task::spawn(async move {
//             match node.initiate_connection(remote_address).await {
//                 Err(NetworkError::PeerAlreadyConnecting) | Err(NetworkError::PeerAlreadyConnected) => {
//                     // no issue here, already connecting
//                 }
//                 Err(e @ NetworkError::TooManyConnections) | Err(e @ NetworkError::SelfConnectAttempt) => {
//                     warn!("Couldn't connect to peer {}: {}", remote_address, e);
//                     // the connection hasn't been established, no need to disconnect
//                 }
//                 Err(e) => {
//                     warn!("Couldn't connect to peer {}: {}", remote_address, e);
//                     node.disconnect_from_peer(remote_address);
//                 }
//                 Ok(_) => {}
//             }
//         });
//
//         // Query the peer for its peers.
//         self.send_request(Message::new(Direction::Outbound(crawl_node), Payload::GetPeers));
//     }
// }

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
