use crate::peer_book::PeerBook;

use crate::Connections;
use std::net::SocketAddr;
use tokio::sync::RwLock;

/// Network context.
pub struct Context {
    /// Tcp stream connections
    pub connections: RwLock<Connections>,

    /// This node is a bootnode
    pub is_bootnode: bool,

    /// Personal socket address
    pub local_addr: SocketAddr,

    /// Peer book
    pub peer_book: RwLock<PeerBook>,

    /// Frequency the server requests memory pool transactions x 10 seconds
    pub memory_pool_interval: u8,

    /// Mininmum number of peers to connect to
    pub min_peers: u16,

    /// Maximum number of peers to connect to
    pub max_peers: u16,

    pub bootnodes: Vec<String>,
}

impl Context {
    pub fn new(
        local_addr: SocketAddr,
        memory_pool_interval: u8,
        min_peers: u16,
        max_peers: u16,
        is_bootnode: bool,
        bootnodes: Vec<String>,
    ) -> Self {
        Self {
            connections: RwLock::new(Connections::new()),
            is_bootnode,
            local_addr,
            peer_book: RwLock::new(PeerBook::new()),
            memory_pool_interval,
            min_peers,
            max_peers,
            bootnodes,
        }
    }
}
