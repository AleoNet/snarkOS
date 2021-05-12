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

use metrics::{GaugeValue, Key, Recorder, Unit};

use std::{
    borrow::Borrow,
    sync::atomic::{AtomicU64, Ordering},
};

pub const INBOUND_ALL_SUCCESSES: &str = "snarkos_inbound_all_successes_total";
pub const INBOUND_ALL_FAILURES: &str = "snarkos_inbound_all_failures_total";
pub const INBOUND_BLOCKS: &str = "snarkos_inbound_blocks_total";
pub const INBOUND_GETBLOCKS: &str = "snarkos_inbound_getblocks_total";
pub const INBOUND_GETMEMORYPOOL: &str = "snarkos_inbound_getmemorypool_total";
pub const INBOUND_GETPEERS: &str = "snarkos_inbound_getpeers_total";
pub const INBOUND_GETSYNC: &str = "snarkos_inbound_getsync_total";
pub const INBOUND_MEMORYPOOL: &str = "snarkos_inbound_memorypool_total";
pub const INBOUND_PEERS: &str = "snarkos_inbound_peers_total";
pub const INBOUND_PINGS: &str = "snarkos_inbound_pings_total";
pub const INBOUND_PONGS: &str = "snarkos_inbound_pongs_total";
pub const INBOUND_SYNCS: &str = "snarkos_inbound_syncs_total";
pub const INBOUND_SYNCBLOCKS: &str = "snarkos_inbound_syncblocks_total";
pub const INBOUND_TRANSACTIONS: &str = "snarkos_inbound_transactions_total";
pub const INBOUND_UNKNOWN: &str = "snarkos_inbound_unknown_total";

pub const OUTBOUND_ALL_SUCCESSES: &str = "snarkos_outbound_all_successes_total";
pub const OUTBOUND_ALL_FAILURES: &str = "snarkos_outbound_all_failures_total";

pub const CONNECTIONS_ALL_ACCEPTED: &str = "snarkos_connections_all_accepted_total";
pub const CONNECTIONS_ALL_INITIATED: &str = "snarkos_connections_all_initiated_total";
pub const CONNECTIONS_ALL_REJECTED: &str = "snarkos_connections_all_rejected_total";

pub const HANDSHAKES_FAILURES_INIT: &str = "snarkos_handshakes_failures_init_total";
pub const HANDSHAKES_FAILURES_RESP: &str = "snarkos_handshakes_failures_resp_total";
pub const HANDSHAKES_SUCCESSES_INIT: &str = "snarkos_handshakes_successes_init_total";
pub const HANDSHAKES_SUCCESSES_RESP: &str = "snarkos_handshakes_successes_resp_total";
pub const HANDSHAKES_TIMEOUTS_INIT: &str = "snarkos_handshakes_timeouts_init_total";
pub const HANDSHAKES_TIMEOUTS_RESP: &str = "snarkos_handshakes_timeouts_resp_total";

pub const QUEUES_INBOUND: &str = "snarkos_queues_inbound_total";
pub const QUEUES_OUTBOUND: &str = "snarkos_queues_outbound_total";

pub const MISC_BLOCKS_MINED: &str = "snarkos_misc_blocks_mined_total";
pub const MISC_DUPLICATE_BLOCKS: &str = "snarkos_misc_duplicate_blocks_total";
pub const MISC_DUPLICATE_SYNC_BLOCKS: &str = "snarkos_misc_duplicate_sync_blocks_total";

pub static NODE_STATS: Stats = Stats::new();

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

impl Stats {
    pub const fn new() -> Self {
        Self {
            inbound: InboundStats::new(),
            outbound: OutboundStats::new(),
            connections: ConnectionStats::new(),
            handshakes: HandshakeStats::new(),
            queues: QueueStats::new(),
            misc: MiscStats::new(),
        }
    }
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

impl InboundStats {
    const fn new() -> Self {
        Self {
            all_successes: AtomicU64::new(0),
            all_failures: AtomicU64::new(0),
            blocks: AtomicU64::new(0),
            getblocks: AtomicU64::new(0),
            getmemorypool: AtomicU64::new(0),
            getpeers: AtomicU64::new(0),
            getsync: AtomicU64::new(0),
            memorypool: AtomicU64::new(0),
            peers: AtomicU64::new(0),
            pings: AtomicU64::new(0),
            pongs: AtomicU64::new(0),
            syncs: AtomicU64::new(0),
            syncblocks: AtomicU64::new(0),
            transactions: AtomicU64::new(0),
            unknown: AtomicU64::new(0),
        }
    }
}

#[derive(Default)]
pub struct OutboundStats {
    /// The number of messages successfully sent by the node.
    pub all_successes: AtomicU64,
    /// The number of messages that failed to be sent to peers.
    pub all_failures: AtomicU64,
}

impl OutboundStats {
    const fn new() -> Self {
        Self {
            all_successes: AtomicU64::new(0),
            all_failures: AtomicU64::new(0),
        }
    }
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

impl ConnectionStats {
    const fn new() -> Self {
        Self {
            all_accepted: AtomicU64::new(0),
            all_initiated: AtomicU64::new(0),
            all_rejected: AtomicU64::new(0),
        }
    }
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

impl HandshakeStats {
    const fn new() -> Self {
        Self {
            failures_init: AtomicU64::new(0),
            failures_resp: AtomicU64::new(0),
            successes_init: AtomicU64::new(0),
            successes_resp: AtomicU64::new(0),
            timeouts_init: AtomicU64::new(0),
            timeouts_resp: AtomicU64::new(0),
        }
    }
}

#[derive(Default)]
pub struct QueueStats {
    /// The number of messages queued in the common inbound channel.
    pub inbound: AtomicU64,
    /// The number of messages queued in the individual outbound channels.
    pub outbound: AtomicU64,
}

impl QueueStats {
    const fn new() -> Self {
        Self {
            inbound: AtomicU64::new(0),
            outbound: AtomicU64::new(0),
        }
    }
}

#[derive(Default)]
pub struct MiscStats {
    /// The number of mined blocks.
    pub blocks_mined: AtomicU64,
    /// The number of duplicate blocks received.
    pub duplicate_blocks: AtomicU64,
    /// The number of duplicate sync blocks received.
    pub duplicate_sync_blocks: AtomicU64,
}

impl MiscStats {
    const fn new() -> Self {
        Self {
            blocks_mined: AtomicU64::new(0),
            duplicate_blocks: AtomicU64::new(0),
            duplicate_sync_blocks: AtomicU64::new(0),
        }
    }
}

impl Recorder for Stats {
    // The following are unused in Stats
    fn register_counter(&self, _key: &Key, _unit: Option<Unit>, _desc: Option<&'static str>) {}

    fn register_gauge(&self, _key: &Key, _unit: Option<Unit>, _desc: Option<&'static str>) {}

    fn register_histogram(&self, _key: &Key, _unit: Option<Unit>, _desc: Option<&'static str>) {}

    fn record_histogram(&self, _key: &Key, _value: f64) {}

    fn increment_counter(&self, key: &Key, value: u64) {
        if let Some(name) = key.name().parts().next() {
            match name.borrow() {
                // inbound
                INBOUND_ALL_SUCCESSES => self.inbound.all_successes.fetch_add(value, Ordering::Relaxed),
                INBOUND_ALL_FAILURES => self.inbound.all_failures.fetch_add(value, Ordering::Relaxed),
                INBOUND_BLOCKS => self.inbound.blocks.fetch_add(value, Ordering::Relaxed),
                INBOUND_GETBLOCKS => self.inbound.getblocks.fetch_add(value, Ordering::Relaxed),
                INBOUND_GETMEMORYPOOL => self.inbound.getmemorypool.fetch_add(value, Ordering::Relaxed),
                INBOUND_GETPEERS => self.inbound.getpeers.fetch_add(value, Ordering::Relaxed),
                INBOUND_GETSYNC => self.inbound.getsync.fetch_add(value, Ordering::Relaxed),
                INBOUND_MEMORYPOOL => self.inbound.memorypool.fetch_add(value, Ordering::Relaxed),
                INBOUND_PEERS => self.inbound.peers.fetch_add(value, Ordering::Relaxed),
                INBOUND_PINGS => self.inbound.pings.fetch_add(value, Ordering::Relaxed),
                INBOUND_PONGS => self.inbound.pongs.fetch_add(value, Ordering::Relaxed),
                INBOUND_SYNCS => self.inbound.syncs.fetch_add(value, Ordering::Relaxed),
                INBOUND_SYNCBLOCKS => self.inbound.syncblocks.fetch_add(value, Ordering::Relaxed),
                INBOUND_TRANSACTIONS => self.inbound.transactions.fetch_add(value, Ordering::Relaxed),
                INBOUND_UNKNOWN => self.inbound.unknown.fetch_add(value, Ordering::Relaxed),
                // outbound
                OUTBOUND_ALL_SUCCESSES => self.outbound.all_successes.fetch_add(value, Ordering::Relaxed),
                OUTBOUND_ALL_FAILURES => self.outbound.all_failures.fetch_add(value, Ordering::Relaxed),
                // connections
                CONNECTIONS_ALL_ACCEPTED => self.connections.all_accepted.fetch_add(value, Ordering::Relaxed),
                CONNECTIONS_ALL_INITIATED => self.connections.all_initiated.fetch_add(value, Ordering::Relaxed),
                CONNECTIONS_ALL_REJECTED => self.connections.all_rejected.fetch_add(value, Ordering::Relaxed),
                // handshakes
                HANDSHAKES_FAILURES_INIT => self.handshakes.failures_init.fetch_add(value, Ordering::Relaxed),
                HANDSHAKES_FAILURES_RESP => self.handshakes.failures_resp.fetch_add(value, Ordering::Relaxed),
                HANDSHAKES_SUCCESSES_INIT => self.handshakes.successes_init.fetch_add(value, Ordering::Relaxed),
                HANDSHAKES_SUCCESSES_RESP => self.handshakes.successes_resp.fetch_add(value, Ordering::Relaxed),
                HANDSHAKES_TIMEOUTS_INIT => self.handshakes.timeouts_init.fetch_add(value, Ordering::Relaxed),
                HANDSHAKES_TIMEOUTS_RESP => self.handshakes.timeouts_resp.fetch_add(value, Ordering::Relaxed),
                // misc
                MISC_BLOCKS_MINED => self.misc.blocks_mined.fetch_add(value, Ordering::Relaxed),
                MISC_DUPLICATE_BLOCKS => self.misc.duplicate_blocks.fetch_add(value, Ordering::Relaxed),
                MISC_DUPLICATE_SYNC_BLOCKS => self.misc.duplicate_sync_blocks.fetch_add(value, Ordering::Relaxed),
                _ => {
                    error!("Metrics key {} wasn't assigned an operation and won't work!", key);
                    0
                }
            };
        } else {
            error!("Metrics key {} wasn't assigned a name and won't work!", key);
        }
    }

    fn update_gauge(&self, key: &Key, value: GaugeValue) {
        if let Some(name) = key.name().parts().next() {
            match value {
                GaugeValue::Increment(value) => {
                    match name.borrow() {
                        // queues
                        QUEUES_INBOUND => self.queues.inbound.fetch_add(value as u64, Ordering::Relaxed),
                        QUEUES_OUTBOUND => self.queues.outbound.fetch_add(value as u64, Ordering::Relaxed),
                        _ => {
                            error!("Metrics key {} wasn't assigned an operation and won't work!", key);
                            0
                        }
                    }
                }
                GaugeValue::Decrement(value) => {
                    match name.borrow() {
                        // queues
                        QUEUES_INBOUND => self.queues.inbound.fetch_sub(value as u64, Ordering::Relaxed),
                        QUEUES_OUTBOUND => self.queues.outbound.fetch_sub(value as u64, Ordering::Relaxed),
                        _ => {
                            error!("Metrics key {} wasn't assigned an operation and won't work!", key);
                            0
                        }
                    }
                }
                GaugeValue::Absolute(_value) => {
                    error!("GaugeValue::Absolute is not used!");
                    0
                }
            };
        } else {
            error!("Metrics key {} wasn't assigned a name and won't work!", key);
        }
    }
}
