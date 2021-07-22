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

pub mod inbound {
    pub const ALL_SUCCESSES: &str = "snarkos_inbound_all_successes_total";
    pub const ALL_FAILURES: &str = "snarkos_inbound_all_failures_total";
    pub const BLOCKS: &str = "snarkos_inbound_blocks_total";
    pub const GETBLOCKS: &str = "snarkos_inbound_getblocks_total";
    pub const GETMEMORYPOOL: &str = "snarkos_inbound_getmemorypool_total";
    pub const GETPEERS: &str = "snarkos_inbound_getpeers_total";
    pub const GETSYNC: &str = "snarkos_inbound_getsync_total";
    pub const MEMORYPOOL: &str = "snarkos_inbound_memorypool_total";
    pub const PEERS: &str = "snarkos_inbound_peers_total";
    pub const PINGS: &str = "snarkos_inbound_pings_total";
    pub const PONGS: &str = "snarkos_inbound_pongs_total";
    pub const SYNCS: &str = "snarkos_inbound_syncs_total";
    pub const SYNCBLOCKS: &str = "snarkos_inbound_syncblocks_total";
    pub const TRANSACTIONS: &str = "snarkos_inbound_transactions_total";
    pub const UNKNOWN: &str = "snarkos_inbound_unknown_total";
}

pub mod outbound {
    pub const ALL_SUCCESSES: &str = "snarkos_outbound_all_successes_total";
    pub const ALL_FAILURES: &str = "snarkos_outbound_all_failures_total";
}

pub mod connections {
    pub const ALL_ACCEPTED: &str = "snarkos_connections_all_accepted_total";
    pub const ALL_INITIATED: &str = "snarkos_connections_all_initiated_total";
    pub const ALL_REJECTED: &str = "snarkos_connections_all_rejected_total";
    pub const CONNECTING: &str = "snarkos_connections_connecting_total";
    pub const CONNECTED: &str = "snarkos_connections_connected_total";
    pub const DISCONNECTED: &str = "snarkos_connections_disconnected_total";
}

pub mod handshakes {
    pub const FAILURES_INIT: &str = "snarkos_handshakes_failures_init_total";
    pub const FAILURES_RESP: &str = "snarkos_handshakes_failures_resp_total";
    pub const SUCCESSES_INIT: &str = "snarkos_handshakes_successes_init_total";
    pub const SUCCESSES_RESP: &str = "snarkos_handshakes_successes_resp_total";
    pub const TIMEOUTS_INIT: &str = "snarkos_handshakes_timeouts_init_total";
    pub const TIMEOUTS_RESP: &str = "snarkos_handshakes_timeouts_resp_total";
}

pub mod queues {
    pub const INBOUND: &str = "snarkos_queues_inbound_total";
    pub const OUTBOUND: &str = "snarkos_queues_outbound_total";
}

pub mod misc {
    pub const BLOCK_HEIGHT: &str = "snarkos_misc_block_height_total";
    pub const BLOCKS_MINED: &str = "snarkos_misc_blocks_mined_total";
    pub const CHAIN_BLOCK_HEIGHT: &str = "snarkos_misc_chain_block_height_total";
    pub const DUPLICATE_BLOCKS: &str = "snarkos_misc_duplicate_blocks_total";
    pub const DUPLICATE_SYNC_BLOCKS: &str = "snarkos_misc_duplicate_sync_blocks_total";
    pub const RPC_REQUESTS: &str = "snarkos_misc_rpc_requests_total";
}
