use crate::peer_book::PeerBook;

use crate::{Connections, Handshakes};
use std::net::SocketAddr;
use tokio::sync::RwLock;

/// Network context.
pub struct Context {
    /// Personal socket address
    pub local_addr: SocketAddr,

    /// Frequency the server requests memory pool transactions x 10 seconds
    pub memory_pool_interval: u8,

    /// Mininmum number of peers to connect to
    pub min_peers: u16,

    /// Maximum number of peers to connect to
    pub max_peers: u16,

    /// This node is a bootnode
    pub is_bootnode: bool,

    /// list of bootnodes
    pub bootnodes: Vec<String>,

    /// Tcp stream connections
    pub connections: RwLock<Connections>,

    /// Peer book
    pub peer_book: RwLock<PeerBook>,

    /// Handshakes with other nodes
    pub handshakes: RwLock<Handshakes>,
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
            local_addr,
            memory_pool_interval,
            min_peers,
            max_peers,
            is_bootnode,
            bootnodes,
            connections: RwLock::new(Connections::new()),
            peer_book: RwLock::new(PeerBook::new()),
            handshakes: RwLock::new(Handshakes::new()),
        }
    }
}
