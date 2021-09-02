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

use metrics_catalogue::*;

use crate::snapshots::{
    NodeConnectionStats,
    NodeHandshakeStats,
    NodeInboundStats,
    NodeInternalRttStats,
    NodeMiscStats,
    NodeOutboundStats,
    NodeQueueStats,
    NodeStats,
};

#[derive(Catalogue)]
#[metric(root, "snarkos", separator = "_")]
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
    /// The node's internal RTT from message received to response sent (in seconds).
    internal_rtt: InternalRtt,
}

impl Stats {
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

#[derive(Catalogue)]
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

#[derive(Catalogue)]
pub struct OutboundStats {
    /// The number of messages successfully sent by the node.
    all_successes: Counter,
    /// The number of messages that failed to be sent to peers.
    all_failures: Counter,
    /// The number of messages that were going to be sent to a peer, but was blocked at the last minute by a cache.
    all_cache_hits: Counter,
}

impl OutboundStats {
    pub fn snapshot(&self) -> NodeOutboundStats {
        NodeOutboundStats {
            all_successes: self.all_successes.read(),
            all_failures: self.all_failures.read(),
        }
    }
}

#[derive(Catalogue)]
pub struct ConnectionStats {
    /// The number of all connections the node has accepted.
    all_accepted: Counter,
    /// The number of all connections the node has initiated.
    all_initiated: Counter,
    /// The number of rejected inbound connection requests.
    all_rejected: Counter,
    /// Number of currently connecting peers.
    #[metric("connecting")]
    connecting_peers: DiscreteGauge,
    /// Number of currently connected peers.
    #[metric("connected")]
    connected_peers: DiscreteGauge,
    /// Number of known disconnected peers.
    #[metric("disconnected")]
    disconnected_peers: DiscreteGauge,
    /// Tracks connection durations (once closed).
    duration: Histogram<60>,
}

impl ConnectionStats {
    pub fn snapshot(&self) -> NodeConnectionStats {
        NodeConnectionStats {
            all_accepted: self.all_accepted.read(),
            all_initiated: self.all_initiated.read(),
            all_rejected: self.all_rejected.read(),
            average_duration: self.duration.average(),
            connecting_peers: self.connecting_peers.read() as u32,
            connected_peers: self.connected_peers.read() as u32,
            disconnected_peers: self.disconnected_peers.read() as u32,
        }
    }
}

#[derive(Catalogue)]
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

#[derive(Catalogue)]
pub struct QueueStats {
    /// The number of queued consensus items.
    consensus: DiscreteGauge,
    /// The number of messages queued in the individual inbound channels.
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
    pub fn snapshot(&self) -> NodeQueueStats {
        NodeQueueStats {
            consensus: self.consensus.read() as u64,
            inbound: self.inbound.read() as u64,
            outbound: self.outbound.read() as u64,
            peer_events: self.peer_events.read() as u64,
            storage: self.storage.read() as u64,
            sync_items: self.sync_items.read() as u64,
        }
    }
}

#[derive(Catalogue)]
pub struct MiscStats {
    block_height: DiscreteGauge,
    /// The number of mined blocks.
    blocks_mined: Counter,
    /// The processing time for a block.
    block_processing_time: Histogram<60>,
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
    pub fn snapshot(&self) -> NodeMiscStats {
        NodeMiscStats {
            block_height: self.block_height.read() as u64,
            blocks_mined: self.blocks_mined.read() as u64,
            block_processing_time: self.block_processing_time.average(),
            duplicate_blocks: self.duplicate_blocks.read() as u64,
            duplicate_sync_blocks: self.duplicate_sync_blocks.read() as u64,
            orphan_blocks: self.orphan_blocks.read() as u64,
            rpc_requests: self.rpc_requests.read() as u64,
        }
    }
}

/// Each histogram holds the last `QUEUE_CAPACITY` (see `metric_types` mod) measurements for internal RTT for the indicated message
/// type. The snapshot produced for the RPC stats is the average RTT for each set.
#[derive(Catalogue)]
pub struct InternalRtt {
    getpeers: Histogram<60>,
    getsync: Histogram<60>,
    getblocks: Histogram<60>,
    getmemorypool: Histogram<60>,
}

impl InternalRtt {
    pub fn snapshot(&self) -> NodeInternalRttStats {
        NodeInternalRttStats {
            getpeers: self.getpeers.average(),
            getsync: self.getsync.average(),
            getblocks: self.getblocks.average(),
            getmemorypool: self.getmemorypool.average(),
        }
    }
}

trait HistogramExt {
    fn average(&self) -> f64;
}

impl<const N: u64> HistogramExt for Histogram<N> {
    #[inline]
    fn average(&self) -> f64 {
        let values = self.read();
        values.iter().sum::<f64>() / (values.len() as f64)
    }
}
