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

use snarkos_environment::{ClientTrial, CurrentNetwork, Environment};

/// The IDs of messages that are accepted by the crawler.
// note: ChallengeRequest and ChallengeResponse are only expected during the handshake.
pub const ACCEPTED_MESSAGE_IDS: &[u16] = &[
    4, // Disconnect
    5, // PeerRequest
    6, // PeerResponse
    7, // Ping
];
/// The interval for revisiting successfully crawled nodes.
pub const CRAWL_INTERVAL_MINS: i64 = 10;
/// The amount of time (in seconds) between database writes containing crawling information.
pub const DB_WRITE_INTERVAL_SECS: u8 = 10;
/// The amount of time (in seconds) between crawling logs.
pub const LOG_INTERVAL_SECS: u64 = 10;
/// The maximum number of peers the crawler can have at a time.
pub const MAXIMUM_NUMBER_OF_PEERS: usize = 1000;
/// The amount of time (in seconds) between peer updates.
pub const PEER_UPDATE_INTERVAL_SECS: u64 = 10;
/// The list of sync nodes to begin crawling with.
pub const SYNC_NODES: &[&str] = <ClientTrial<CurrentNetwork>>::SYNC_NODES;
/// Connections that haven't been seen within this time (in hours) are forgotten.
pub const STALE_CONNECTION_CUTOFF_TIME_HRS: i64 = 4;
/// The number of lists of peers that a single peer needs to provide before it's disconnected from.
pub const DESIRED_PEER_SET_COUNT: u8 = 3;
/// The maximum number of connections to attempt to initiate when updating peers.
pub const NUM_CONCURRENT_CONNECTION_ATTEMPTS: u8 = 50;
/// The number of peers shared with the crawled nodes when requested.
pub const SHARED_PEER_COUNT: usize = 15;
/// The number of connection failures after which a node is removed from the list of known nodes.
pub const MAX_CONNECTION_FAILURE_COUNT: u8 = 10;
