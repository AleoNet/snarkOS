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

use serde::{Deserialize, Serialize};

/// Returned value for the `getnodestats` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeStats {
    /// Stats related to messages received by the node.
    pub inbound: NodeInboundStats,
    /// Stats related to messages sent by the node.
    pub outbound: NodeOutboundStats,
    /// Stats related to the node's connections.
    pub connections: NodeConnectionStats,
    /// Stats related to the node's handshakes.
    pub handshakes: NodeHandshakeStats,
    /// Stats related to the node's queues.
    pub queues: NodeQueueStats,
    /// Miscellaneous stats related to the node.
    pub misc: NodeMiscStats,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeInboundStats {
    /// The number of successfully processed inbound messages.
    pub all_successes: u64,
    /// The number of inbound messages that couldn't be processed.
    pub all_failures: u64,
    /// The number of all received `Block` messages.
    pub blocks: u64,
    /// The number of all received `GetBlocks` messages.
    pub getblocks: u64,
    /// The number of all received `GetMemoryPool` messages.
    pub getmemorypool: u64,
    /// The number of all received `GetPeers` messages.
    pub getpeers: u64,
    /// The number of all received `GetSync` messages.
    pub getsync: u64,
    /// The number of all received `MemoryPool` messages.
    pub memorypool: u64,
    /// The number of all received `Peers` messages.
    pub peers: u64,
    /// The number of all received `Ping` messages.
    pub pings: u64,
    /// The number of all received `Pong` messages.
    pub pongs: u64,
    /// The number of all received `Sync` messages.
    pub syncs: u64,
    /// The number of all received `SyncBlock` messages.
    pub syncblocks: u64,
    /// The number of all received `Transaction` messages.
    pub transactions: u64,
    /// The number of all received `Unknown` messages.
    pub unknown: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeOutboundStats {
    /// The number of messages successfully sent by the node.
    pub all_successes: u64,
    /// The number of messages that failed to be sent to peers.
    pub all_failures: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeConnectionStats {
    /// The number of all connections the node has accepted.
    pub all_accepted: u64,
    /// The number of all connections the node has initiated.
    pub all_initiated: u64,
    /// The number of rejected inbound connection requests.
    pub all_rejected: u64,
    /// Number of currently connecting peers.
    pub connecting_peers: u32,
    /// Number of currently connected peers.
    pub connected_peers: u32,
    /// Number of known disconnected peers.
    pub disconnected_peers: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeHandshakeStats {
    /// The number of failed handshakes as the initiator.
    pub failures_init: u64,
    /// The number of failed handshakes as the responder.
    pub failures_resp: u64,
    /// The number of successful handshakes as the initiator.
    pub successes_init: u64,
    /// The number of successful handshakes as the responder.
    pub successes_resp: u64,
    /// The number of handshake timeouts as the initiator.
    pub timeouts_init: u64,
    /// The number of handshake timeouts as the responder.
    pub timeouts_resp: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeQueueStats {
    /// The number of messages queued in the common inbound channel.
    pub inbound: u64,
    /// The number of messages queued in the individual outbound channels.
    pub outbound: u64,
    /// The number of queued peer events.
    pub peer_events: u64,
    /// The number of queued storage requests.
    pub storage: u64,
    /// The number of queued sync items.
    pub sync_items: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeMiscStats {
    /// The current block height of the node.
    pub block_height: u64,
    /// The number of blocks the node has mined.
    pub blocks_mined: u64,
    /// The number of duplicate blocks received.
    pub duplicate_blocks: u64,
    /// The number of duplicate sync blocks received.
    pub duplicate_sync_blocks: u64,
    /// The number of orphan blocks received.
    pub orphan_blocks: u64,
    /// The number of RPC requests received.
    pub rpc_requests: u64,
}
