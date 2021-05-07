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

use std::sync::atomic::{AtomicU32, AtomicU64};

// TODO: make members private and make gathering of stats feature-gated and possibly
// interchangeable with prometheus metrics.
#[derive(Default)]
pub struct Stats {
    /// Stats related to messages received by the node.
    pub inbound: InboundStats,
    /// Stats related to messages sent by the node.
    pub outbound: OutboundStats,
    /// Stats related to the node's connections.
    pub connections: ConnectionStats,
    /// Stats related to the node's handshakes.
    pub handshakes: HandshakeStats,
    /// Stats related to the node's queues.
    pub queues: QueueStats,
    /// Miscellaneous stats related to the node.
    pub misc: MiscStats,
}

#[derive(Default)]
pub struct InboundStats {
    /// The number of successfully processed inbound messages.
    pub all_successes: AtomicU64,
    /// The number of inbound messages that couldn't be processed.
    pub all_failures: AtomicU64,

    /// The number of all received `Block` messages.
    pub blocks: AtomicU64,
    /// The number of all received `GetBlocks` messages.
    pub getblocks: AtomicU64,
    /// The number of all received `GetMemoryPool` messages.
    pub getmemorypool: AtomicU64,
    /// The number of all received `GetPeers` messages.
    pub getpeers: AtomicU64,
    /// The number of all received `GetSync` messages.
    pub getsync: AtomicU64,
    /// The number of all received `MemoryPool` messages.
    pub memorypool: AtomicU64,
    /// The number of all received `Peers` messages.
    pub peers: AtomicU64,
    /// The number of all received `Ping` messages.
    pub pings: AtomicU64,
    /// The number of all received `Pong` messages.
    pub pongs: AtomicU64,
    /// The number of all received `Sync` messages.
    pub syncs: AtomicU64,
    /// The number of all received `SyncBlock` messages.
    pub syncblocks: AtomicU64,
    /// The number of all received `Transaction` messages.
    pub transactions: AtomicU64,
    /// The number of all received `Unknown` messages.
    pub unknown: AtomicU64,
}

#[derive(Default)]
pub struct OutboundStats {
    /// The number of messages successfully sent by the node.
    pub all_successes: AtomicU64,
    /// The number of messages that failed to be sent to peers.
    pub all_failures: AtomicU64,
}

#[derive(Default)]
pub struct ConnectionStats {
    /// The number of all connections the node has accepted.
    pub all_accepted: AtomicU64,
    /// The number of all connections the node has initiated.
    pub all_initiated: AtomicU64,
    /// The number of rejected inbound connection requests.
    pub all_rejected: AtomicU64,
}

#[derive(Default)]
pub struct HandshakeStats {
    /// The number of failed handshakes as the initiator.
    pub failures_init: AtomicU64,
    /// The number of failed handshakes as the responder.
    pub failures_resp: AtomicU64,
    /// The number of successful handshakes as the initiator.
    pub successes_init: AtomicU64,
    /// The number of successful handshakes as the responder.
    pub successes_resp: AtomicU64,
    /// The number of handshake timeouts as the initiator.
    pub timeouts_init: AtomicU64,
    /// The number of handshake timeouts as the responder.
    pub timeouts_resp: AtomicU64,
}

#[derive(Default)]
pub struct QueueStats {
    /// The number of messages queued in the common inbound channel.
    pub inbound: AtomicU32,
    /// The number of messages queued in the individual outbound channels.
    pub outbound: AtomicU32,
}

#[derive(Default)]
pub struct MiscStats {
    /// The number of mined blocks.
    pub blocks_mined: AtomicU32,
    /// The number of duplicate blocks received.
    pub duplicate_blocks: AtomicU64,
    /// The number of duplicate sync blocks received.
    pub duplicate_sync_blocks: AtomicU64,
}
