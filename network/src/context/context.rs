use crate::{Connections, Handshakes, PeerBook, Pings, SyncHandler};

use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{Mutex, RwLock};

/// The network context for this node.
/// All variables are public to allow server components to acquire read/write access.
pub struct Context {
    /// The ip address/socket of this node.
    pub local_address: SocketAddr,

    /// Protocol version number
    pub version: u64,

    /// Frequency the server connection handler sends messages to connected peers.
    pub connection_frequency: u64,

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

    /// Block syncing protocol handler
    pub sync_handler: Arc<Mutex<SyncHandler>>,
}

impl Context {
    /// Construct a new network `Context`.
    pub fn new(
        local_address: SocketAddr,
        version: u64,
        connection_frequency: u64,
        memory_pool_interval: u8,
        min_peers: u16,
        max_peers: u16,
        is_bootnode: bool,
        bootnodes: Vec<String>,
    ) -> Self {
        let mut bootnode = local_address;
        if !is_bootnode && !bootnodes.is_empty() {
            bootnode = bootnodes[0].parse::<SocketAddr>().expect("Invalid bootnode in config");
        }

        Self {
            local_address,
            version,
            connection_frequency,
            memory_pool_interval,
            min_peers,
            max_peers,
            is_bootnode,
            bootnodes,
            connections: RwLock::new(Connections::new()),
            peer_book: RwLock::new(PeerBook::new(local_address)),
            handshakes: RwLock::new(Handshakes::new(version)),
            pings: RwLock::new(Pings::new()),
            sync_handler: Arc::new(Mutex::new(SyncHandler::new(bootnode))),
        }
    }
}
