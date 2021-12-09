use parking_lot::RwLock;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    net::SocketAddr,
};
use time::OffsetDateTime;

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
    pub last_seen: OffsetDateTime,
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
    pub fn new(source: SocketAddr, target: SocketAddr) -> Self {
        Connection {
            source,
            target,
            last_seen: OffsetDateTime::now_utc(),
        }
    }
}

/// Constructs a set of nodes contained from the connection set.
pub fn nodes_from_connections(connections: &HashSet<Connection>) -> HashSet<SocketAddr> {
    let mut nodes: HashSet<SocketAddr> = HashSet::new();
    for connection in connections.iter() {
        // Using a hashset guarantees uniqueness.
        nodes.insert(connection.source);
        nodes.insert(connection.target);
    }

    nodes
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn connections_partial_eq() {
        let a = "12.34.56.78:9000".parse().unwrap();
        let b = "98.76.54.32:1000".parse().unwrap();

        assert_eq!(Connection::new(a, b), Connection::new(b, a));
        assert_eq!(Connection::new(a, b), Connection::new(a, b));
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
}
