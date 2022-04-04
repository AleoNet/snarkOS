// Copyright (C) 2019-2022 Aleo Systems Inc.
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

pub const GAUGES: [&str; 4] = [blocks::HEIGHT, peers::CONNECTED, peers::CANDIDATE, peers::RESTRICTED];
pub const HISTOGRAMS: [&str; 4] = [
    internal_rtt::PING,
    internal_rtt::PONG,
    internal_rtt::PEER_REQUEST,
    internal_rtt::BLOCK_REQUEST,
];
pub const COUNTERS: [&str; 9] = [
    message_counts::PING,
    message_counts::PONG,
    message_counts::PEER_REQUEST,
    message_counts::PEER_RESPONSE,
    message_counts::BLOCK_REQUEST,
    message_counts::BLOCK_RESPONSE,
    message_counts::UNCONFIRMED_BLOCK,
    message_counts::UNCONFIRMED_TRANSACTION,
    message_counts::DISCONNECT,
];

pub mod blocks {
    pub const HEIGHT: &str = "snarkos_blocks_height_total";
}

pub mod peers {
    pub const CONNECTED: &str = "snarkos_peers_connected_total";
    pub const CANDIDATE: &str = "snarkos_peers_candidate_total";
    pub const RESTRICTED: &str = "snarkos_peers_restricted_total";
}

pub mod internal_rtt {
    pub const PING: &str = "snarkos_internal_rtt_ping";
    pub const PONG: &str = "snarkos_internal_rtt_pong";
    pub const PEER_REQUEST: &str = "snarkos_internal_rtt_peer_request";
    pub const BLOCK_REQUEST: &str = "snarkos_internal_rtt_block_request";
}

pub mod message_counts {
    pub const PING: &str = "snarkos_message_counts_ping";
    pub const PONG: &str = "snarkos_message_counts_pong";
    pub const PEER_REQUEST: &str = "snarkos_message_counts_peer_request";
    pub const PEER_RESPONSE: &str = "snarkos_message_counts_peer_response";
    pub const BLOCK_REQUEST: &str = "snarkos_message_counts_block_request";
    pub const BLOCK_RESPONSE: &str = "snarkos_message_counts_block_response";
    pub const UNCONFIRMED_BLOCK: &str = "snarkos_message_counts_unconfirmed_block";
    pub const UNCONFIRMED_TRANSACTION: &str = "snarkos_message_counts_unconfirmed_transaction";
    pub const DISCONNECT: &str = "snarkos_message_counts_disconnect";
}
