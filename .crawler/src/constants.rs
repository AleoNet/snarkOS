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

use snarkos_environment::{Client, CurrentNetwork, Environment};

pub const PING_INTERVAL_SECS: u64 = 5;
pub const PEER_INTERVAL_SECS: u64 = 3;
pub const DESIRED_CONNECTIONS: usize = <Client<CurrentNetwork>>::MINIMUM_NUMBER_OF_PEERS * 3;
pub const SYNC_NODES: &'static [&'static str] = <Client<CurrentNetwork>>::SYNC_NODES;
pub const MAXIMUM_NUMBER_OF_PEERS: usize = 10000;
pub const MESSAGE_VERSION: u32 = <Client<CurrentNetwork>>::MESSAGE_VERSION;
