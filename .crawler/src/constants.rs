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

use snarkos_environment::{Client, ClientTrial, CurrentNetwork, Environment};

pub const ACCEPTED_MESSAGE_IDS: &'static [u16] = &[
    2, // ChallengeRequest
    3, // ChallengeResponse
    4, // Disconnect
    5, // PeerRequest
    6, // PeerResponse
    7, // Ping
];
// The interval for revisiting connections.
pub const CRAWL_INTERVAL_MINS: i64 = 3;
pub const LOG_INTERVAL_SECS: u64 = 12;
pub const MAXIMUM_NUMBER_OF_PEERS: usize = 1000;
pub const MESSAGE_VERSION: u32 = <Client<CurrentNetwork>>::MESSAGE_VERSION;
pub const PEER_INTERVAL_SECS: u64 = 10;
pub const SYNC_NODES: &'static [&'static str] = <ClientTrial<CurrentNetwork>>::SYNC_NODES;
// Purges connections that haven't been seen within this time (in hours).
pub const STALE_CONNECTION_CUTOFF_TIME_HRS: i64 = 4;
pub const DESIRED_PEER_SET_COUNT: u8 = 3;
