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

    /// The number of mined blocks.
    pub blocks_mined: AtomicU32,
}

#[derive(Default)]
pub struct InboundStats {
    /// The number of successfully processed inbound messages.
    pub all_successes: AtomicU64,
    /// The number of inbound messages that couldn't be processed.
    pub all_failures: AtomicU64,

    /// The current number of messages queued in the inbound channel.
    pub queued_messages: AtomicU64,

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
}
