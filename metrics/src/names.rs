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
    pub const BLOCK_REQUESTS: &str = "snarkos_inbound_block_requests_total";
    pub const BLOCK_RESPONSES: &str = "snarkos_inbound_block_responses_total";
    pub const CHALLENGE_REQUESTS: &str = "snarkos_inbound_challenge_requests_total";
    pub const CHALLENGE_RESPONSES: &str = "snarkos_inbound_challenge_responses_total";
    pub const DISCONNECTS: &str = "snarkos_inbound_disconnects_total";
    pub const PEER_REQUESTS: &str = "snarkos_inbound_peer_requests_total";
    pub const PEER_RESPONSES: &str = "snarkos_inbound_peer_responses_total";
    pub const PINGS: &str = "snarkos_inbound_pings_total";
    pub const PONGS: &str = "snarkos_inbound_pongs_total";
    pub const UNCONFIRMED_BLOCKS: &str = "snarkos_inbound_unconfirmed_blocks_total";
    pub const UNCONFIRMED_TRANSACTIONS: &str = "snarkos_inbound_unconfirmed_transactions_total";
    pub const UNUSED: &str = "snarkos_inbound_unused_total";
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

pub mod blocks {
    pub const HEIGHT: &str = "snarkos_blocks_height_total";
    pub const MINED: &str = "snarkos_blocks_mined_total";
    pub const DUPLICATES: &str = "snarkos_blocks_duplicates_total";
    pub const ORPHANS: &str = "snarkos_blocks_orphan_total";
}
