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

use crate::{
    metric_types::{CircularHistogram, Counter, DiscreteGauge},
    names::*,
    snapshots::{
        NodeConnectionStats,
        NodeHandshakeStats,
        NodeInboundStats,
        NodeInternalRttStats,
        NodeMiscStats,
        NodeOutboundStats,
        NodeQueueStats,
        NodeStats,
    },
};

pub static NODE_STATS: Stats = Stats::new();

pub struct Stats {
    /// Stats related to messages received by the node.
    inbound: InboundStats,
    /// Stats related to messages sent by the node.
    outbound: OutboundStats,
    /// Stats related to the node's connections.
    connections: ConnectionStats,
    /// Stats related to the node's handshakes.
    handshakes: HandshakeStats,
    /// Stats related to the node's queues.
    queues: QueueStats,
    /// Miscellaneous stats related to the node.
    misc: MiscStats,
    /// The node's internal RTT from message received to response sent.
    internal_rtt: InternalRtt,
}

impl Stats {
    const fn new() -> Self {
        Self {
            inbound: InboundStats::new(),
            outbound: OutboundStats::new(),
            connections: ConnectionStats::new(),
            handshakes: HandshakeStats::new(),
            queues: QueueStats::new(),
            misc: MiscStats::new(),
            internal_rtt: InternalRtt::new(),
        }
    }

    pub fn snapshot(&self) -> NodeStats {
        NodeStats {
            inbound: self.inbound.snapshot(),
            outbound: self.outbound.snapshot(),
            connections: self.connections.snapshot(),
            handshakes: self.handshakes.snapshot(),
            queues: self.queues.snapshot(),
            misc: self.misc.snapshot(),
            internal_rtt: self.internal_rtt.snapshot(),
        }
    }
}

pub struct InboundStats {
    /// The number of successfully processed inbound messages.
    all_successes: Counter,
    /// The number of inbound messages that couldn't be processed.
    all_failures: Counter,
    /// The number of all received `Block` messages.
    blocks: Counter,
    /// The number of all received `GetBlocks` messages.
    getblocks: Counter,
    /// The number of all received `GetMemoryPool` messages.
    getmemorypool: Counter,
    /// The number of all received `GetPeers` messages.
    getpeers: Counter,
    /// The number of all received `GetSync` messages.
    getsync: Counter,
    /// The number of all received `MemoryPool` messages.
    memorypool: Counter,
    /// The number of all received `Peers` messages.
    peers: Counter,
    /// The number of all received `Ping` messages.
    pings: Counter,
    /// The number of all received `Pong` messages.
    pongs: Counter,
    /// The number of all received `Sync` messages.
    syncs: Counter,
    /// The number of all received `SyncBlock` messages.
    syncblocks: Counter,
    /// The number of all received `Transaction` messages.
    transactions: Counter,
    /// The number of all received `Unknown` messages.
    unknown: Counter,
}

impl InboundStats {
    const fn new() -> Self {
        Self {
            all_successes: Counter::new(),
            all_failures: Counter::new(),
            blocks: Counter::new(),
            getblocks: Counter::new(),
            getmemorypool: Counter::new(),
            getpeers: Counter::new(),
            getsync: Counter::new(),
            memorypool: Counter::new(),
            peers: Counter::new(),
            pings: Counter::new(),
            pongs: Counter::new(),
            syncs: Counter::new(),
            syncblocks: Counter::new(),
            transactions: Counter::new(),
            unknown: Counter::new(),
        }
    }

    pub fn snapshot(&self) -> NodeInboundStats {
        NodeInboundStats {
            all_successes: self.all_successes.read(),
            all_failures: self.all_failures.read(),
            blocks: self.blocks.read(),
            getblocks: self.getblocks.read(),
            getmemorypool: self.getmemorypool.read(),
            getpeers: self.getpeers.read(),
            getsync: self.getsync.read(),
            memorypool: self.memorypool.read(),
            peers: self.peers.read(),
            pings: self.pings.read(),
            pongs: self.pongs.read(),
            syncs: self.syncs.read(),
            syncblocks: self.syncblocks.read(),
            transactions: self.transactions.read(),
            unknown: self.unknown.read(),
        }
    }
}

pub struct OutboundStats {
    /// The number of messages successfully sent by the node.
    all_successes: Counter,
    /// The number of messages that failed to be sent to peers.
    all_failures: Counter,
}

impl OutboundStats {
    const fn new() -> Self {
        Self {
            all_successes: Counter::new(),
            all_failures: Counter::new(),
        }
    }

    pub fn snapshot(&self) -> NodeOutboundStats {
        NodeOutboundStats {
            all_successes: self.all_successes.read(),
            all_failures: self.all_failures.read(),
        }
    }
}

pub struct ConnectionStats {
    /// The number of all connections the node has accepted.
    all_accepted: Counter,
    /// The number of all connections the node has initiated.
    all_initiated: Counter,
    /// The number of rejected inbound connection requests.
    all_rejected: Counter,
    /// Number of currently connecting peers.
    connecting_peers: DiscreteGauge,
    /// Number of currently connected peers.
    connected_peers: DiscreteGauge,
    /// Number of known disconnected peers.
    disconnected_peers: DiscreteGauge,
}

impl ConnectionStats {
    const fn new() -> Self {
        Self {
            all_accepted: Counter::new(),
            all_initiated: Counter::new(),
            all_rejected: Counter::new(),
            connecting_peers: DiscreteGauge::new(),
            connected_peers: DiscreteGauge::new(),
            disconnected_peers: DiscreteGauge::new(),
        }
    }

    pub fn snapshot(&self) -> NodeConnectionStats {
        NodeConnectionStats {
            all_accepted: self.all_accepted.read(),
            all_initiated: self.all_initiated.read(),
            all_rejected: self.all_rejected.read(),
            connecting_peers: self.connecting_peers.read() as u32,
            connected_peers: self.connected_peers.read() as u32,
            disconnected_peers: self.disconnected_peers.read() as u32,
        }
    }
}

pub struct HandshakeStats {
    /// The number of failed handshakes as the initiator.
    failures_init: Counter,
    /// The number of failed handshakes as the responder.
    failures_resp: Counter,
    /// The number of successful handshakes as the initiator.
    successes_init: Counter,
    /// The number of successful handshakes as the responder.
    successes_resp: Counter,
    /// The number of handshake timeouts as the initiator.
    timeouts_init: Counter,
    /// The number of handshake timeouts as the responder.
    timeouts_resp: Counter,
}

impl HandshakeStats {
    const fn new() -> Self {
        Self {
            failures_init: Counter::new(),
            failures_resp: Counter::new(),
            successes_init: Counter::new(),
            successes_resp: Counter::new(),
            timeouts_init: Counter::new(),
            timeouts_resp: Counter::new(),
        }
    }

    pub fn snapshot(&self) -> NodeHandshakeStats {
        NodeHandshakeStats {
            successes_init: self.successes_init.read(),
            successes_resp: self.successes_resp.read(),
            failures_init: self.failures_init.read(),
            failures_resp: self.failures_resp.read(),
            timeouts_init: self.timeouts_init.read(),
            timeouts_resp: self.timeouts_resp.read(),
        }
    }
}

pub struct QueueStats {
    /// The number of queued consensus items.
    consensus: DiscreteGauge,
    /// The number of messages queued in the common inbound channel.
    inbound: DiscreteGauge,
    /// The number of messages queued in the individual outbound channels.
    outbound: DiscreteGauge,
    /// The number of queued peer events.
    peer_events: DiscreteGauge,
    /// The number of queued storage requests.
    storage: DiscreteGauge,
    /// The number of queued sync items.
    sync_items: DiscreteGauge,
}

impl QueueStats {
    const fn new() -> Self {
        Self {
            consensus: DiscreteGauge::new(),
            inbound: DiscreteGauge::new(),
            outbound: DiscreteGauge::new(),
            peer_events: DiscreteGauge::new(),
            storage: DiscreteGauge::new(),
            sync_items: DiscreteGauge::new(),
        }
    }

    pub fn snapshot(&self) -> NodeQueueStats {
        NodeQueueStats {
            consensus: self.consensus.read(),
            inbound: self.inbound.read(),
            outbound: self.outbound.read(),
            peer_events: self.peer_events.read(),
            storage: self.storage.read(),
            sync_items: self.sync_items.read(),
        }
    }
}

pub struct MiscStats {
    block_height: DiscreteGauge,
    /// The number of mined blocks.
    blocks_mined: Counter,
    /// The number of duplicate blocks received.
    duplicate_blocks: Counter,
    /// The number of duplicate sync blocks received.
    duplicate_sync_blocks: Counter,
    /// The number of orphan blocks received.
    orphan_blocks: Counter,
    /// The number of RPC requests received.
    rpc_requests: Counter,
}

impl MiscStats {
    const fn new() -> Self {
        Self {
            block_height: DiscreteGauge::new(),
            blocks_mined: Counter::new(),
            duplicate_blocks: Counter::new(),
            duplicate_sync_blocks: Counter::new(),
            orphan_blocks: Counter::new(),
            rpc_requests: Counter::new(),
        }
    }

    pub fn snapshot(&self) -> NodeMiscStats {
        NodeMiscStats {
            block_height: self.block_height.read(),
            blocks_mined: self.blocks_mined.read(),
            duplicate_blocks: self.duplicate_blocks.read(),
            duplicate_sync_blocks: self.duplicate_sync_blocks.read(),
            orphan_blocks: self.orphan_blocks.read(),
            rpc_requests: self.rpc_requests.read(),
        }
    }
}

/// Each histogram holds the last 1024 measurements for internal RTT for the indicated message
/// type. The snapshot produced for the RPC stats is the average RTT for each set.
pub struct InternalRtt {
    getpeers: CircularHistogram,
    getsync: CircularHistogram,
    getblocks: CircularHistogram,
    getmemorypool: CircularHistogram,
}

impl InternalRtt {
    const fn new() -> Self {
        Self {
            getpeers: CircularHistogram::new(),
            getsync: CircularHistogram::new(),
            getblocks: CircularHistogram::new(),
            getmemorypool: CircularHistogram::new(),
        }
    }

    pub fn snapshot(&self) -> NodeInternalRttStats {
        NodeInternalRttStats {
            getpeers: self.getpeers.average(),
            getsync: self.getsync.average(),
            getblocks: self.getblocks.average(),
            getmemorypool: self.getmemorypool.average(),
        }
    }
}

impl Recorder for Stats {
    // The following are unused in Stats
    fn register_counter(&self, _key: &Key, _unit: Option<Unit>, _desc: Option<&'static str>) {}

    fn register_gauge(&self, _key: &Key, _unit: Option<Unit>, _desc: Option<&'static str>) {}

    fn register_histogram(&self, _key: &Key, _unit: Option<Unit>, _desc: Option<&'static str>) {}

    fn record_histogram(&self, key: &Key, value: f64) {
        let metric = match key.name() {
            internal_rtt::GETPEERS => &self.internal_rtt.getpeers,
            internal_rtt::GETSYNC => &self.internal_rtt.getsync,
            internal_rtt::GETBLOCKS => &self.internal_rtt.getblocks,
            internal_rtt::GETMEMORYPOOL => &self.internal_rtt.getmemorypool,
            _ => return,
        };

        metric.push(value);
    }

    fn increment_counter(&self, key: &Key, value: u64) {
        let metric = match key.name() {
            // inbound
            inbound::ALL_SUCCESSES => &self.inbound.all_successes,
            inbound::ALL_FAILURES => &self.inbound.all_failures,
            inbound::BLOCKS => &self.inbound.blocks,
            inbound::GETBLOCKS => &self.inbound.getblocks,
            inbound::GETMEMORYPOOL => &self.inbound.getmemorypool,
            inbound::GETPEERS => &self.inbound.getpeers,
            inbound::GETSYNC => &self.inbound.getsync,
            inbound::MEMORYPOOL => &self.inbound.memorypool,
            inbound::PEERS => &self.inbound.peers,
            inbound::PINGS => &self.inbound.pings,
            inbound::PONGS => &self.inbound.pongs,
            inbound::SYNCS => &self.inbound.syncs,
            inbound::SYNCBLOCKS => &self.inbound.syncblocks,
            inbound::TRANSACTIONS => &self.inbound.transactions,
            inbound::UNKNOWN => &self.inbound.unknown,
            // outbound
            outbound::ALL_SUCCESSES => &self.outbound.all_successes,
            outbound::ALL_FAILURES => &self.outbound.all_failures,
            // connections
            connections::ALL_ACCEPTED => &self.connections.all_accepted,
            connections::ALL_INITIATED => &self.connections.all_initiated,
            connections::ALL_REJECTED => &self.connections.all_rejected,
            // handshakes
            handshakes::FAILURES_INIT => &self.handshakes.failures_init,
            handshakes::FAILURES_RESP => &self.handshakes.failures_resp,
            handshakes::SUCCESSES_INIT => &self.handshakes.successes_init,
            handshakes::SUCCESSES_RESP => &self.handshakes.successes_resp,
            handshakes::TIMEOUTS_INIT => &self.handshakes.timeouts_init,
            handshakes::TIMEOUTS_RESP => &self.handshakes.timeouts_resp,
            // misc
            misc::BLOCKS_MINED => &self.misc.blocks_mined,
            misc::DUPLICATE_BLOCKS => &self.misc.duplicate_blocks,
            misc::DUPLICATE_SYNC_BLOCKS => &self.misc.duplicate_sync_blocks,
            misc::ORPHAN_BLOCKS => &self.misc.orphan_blocks,
            misc::RPC_REQUESTS => &self.misc.rpc_requests,
            _ => {
                return;
            }
        };
        metric.increment(value);
    }

    fn update_gauge(&self, key: &Key, value: GaugeValue) {
        let metric = match key.name() {
            // queues
            queues::INBOUND => &self.queues.inbound,
            queues::OUTBOUND => &self.queues.outbound,
            queues::PEER_EVENTS => &self.queues.peer_events,
            queues::STORAGE => &self.queues.storage,
            // misc
            misc::BLOCK_HEIGHT => &self.misc.block_height,
            // connections
            connections::CONNECTING => &self.connections.connecting_peers,
            connections::CONNECTED => &self.connections.connected_peers,
            connections::DISCONNECTED => &self.connections.disconnected_peers,
            _ => {
                return;
            }
        };
        match value {
            GaugeValue::Increment(val) => metric.increase(val),
            GaugeValue::Decrement(val) => metric.decrease(val),
            GaugeValue::Absolute(val) => metric.set(val),
        }
    }
}
