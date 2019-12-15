//! Definitions of network messages.
use snarkos_objects::BlockHeaderHash;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Message {
    /// One of our peers has found a new block
    Block {
        /// block data
        block_serialized: Vec<u8>,
    },

    /// Request a block from a peer
    BlockRequest {
        /// Block header hash of block being requested
        block_hash: BlockHeaderHash,
    },

    /// Request a peer's memory pool
    MemoryPoolRequest,

    /// Send memory pool
    MemoryPoolResponse {
        /// Memory pool transactions
        memory_pool_transactions: Vec<Vec<u8>>,
    },

    /// Request connected peers
    PeersRequest,

    /// Send peer addresses back
    PeersResponse {
        /// Addresses to share
        addresses: HashMap<SocketAddr, DateTime<Utc>>,
    },

    /// Ping message for maintaining connections
    Ping,

    /// Pong message for maintaining connections
    Pong,

    /// A verified block from our miner or a peer
    PropagateBlock {
        /// block data
        block_serialized: Vec<u8>,
    },

    /// Reject a connection
    Reject,

    /// Sync Node is sending blocks
    SyncBlock {
        /// block data
        block_serialized: Vec<u8>,
    },

    /// Request all blocks from peer
    SyncRequest {
        /// Block header hash of last block received
        block_locator_hashes: Vec<BlockHeaderHash>,
    },

    /// Send block hashes
    SyncResponse {
        /// Block hashes to share
        block_hashes: Vec<BlockHeaderHash>,
    },

    /// One of our peers has a new transaction
    Transaction {
        /// transaction bytes
        transaction_bytes: Vec<u8>,
    },

    /// A Verack message
    Verack,

    /// A `version` message.
    Version {
        /// The network version number
        version: u64,

        /// Message timestamp
        timestamp: DateTime<Utc>,

        /// Latest block number of node sending this message
        height: u32,

        /// Network address of message recipient
        address_receiver: SocketAddr,
    },
}
