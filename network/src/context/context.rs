use crate::{Connections, Handshakes, PeerBook, Pings};

use std::net::SocketAddr;
use tokio::sync::RwLock;

/// The network context for this node.
pub struct Context {
    /// The ip address/socket of this node.
    pub local_address: SocketAddr,

    /// Frequency the server requests memory pool transactions.
    pub memory_pool_interval: u8,

    /// Mininmum number of peers to connect to
    pub min_peers: u16,

    /// Maximum number of peers to connect to
    pub max_peers: u16,

    /// If enabled, node will not connect to bootnodes on startup.
    pub is_bootnode: bool,

    /// Hardcoded nodes and user-specified nodes this node should connect to on startup.
    pub bootnodes: Vec<String>,

    /// Manages connected, gossiped, and disconnected peers
    pub peer_book: RwLock<PeerBook>,

    /// Handshakes to make connected peers
    pub handshakes: RwLock<Handshakes>,

    /// Connected peer channels for reading/writing messages
    pub connections: RwLock<Connections>,

    /// Ping/pongs with connected peers
    pub pings: RwLock<Pings>,
}

impl Context {
    /// Construct a new network `Context`.
    pub fn new(
        local_address: SocketAddr,
        memory_pool_interval: u8,
        min_peers: u16,
        max_peers: u16,
        is_bootnode: bool,
        bootnodes: Vec<String>,
    ) -> Self {
        Self {
            local_address,
            memory_pool_interval,
            min_peers,
            max_peers,
            is_bootnode,
            bootnodes,
            connections: RwLock::new(Connections::new()),
            peer_book: RwLock::new(PeerBook::new()),
            handshakes: RwLock::new(Handshakes::new()),
            pings: RwLock::new(Pings::new()),
        }
    }
}
