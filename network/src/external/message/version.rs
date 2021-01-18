// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/version.md"))]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Version {
    /// The version number of the sender's node server.
    pub version: u64,
    /// The random nonce of the connection request.
    pub nonce: u64,
    /// The listening port of the sender.
    pub listening_port: u16,
    /// The timestamp of this message.
    pub timestamp: i64,
}

impl Version {
    pub fn new(version: u64, nonce: u64, listening_port: u16) -> Self {
        Self {
            version,
            nonce,
            listening_port,
            timestamp: Utc::now().timestamp(),
        }
    }

    // currently used for the handshakes, but it's a stop-gap; TODO(ljedrz): replace with a solution that
    // is bound to a setup that also encrypts the post-handshake communication
    pub fn new_with_rng(version: u64, listening_port: u16) -> Self {
        let mut rng = rand::thread_rng();

        Self {
            version,
            nonce: rng.gen::<u64>(),
            listening_port,
            timestamp: Utc::now().timestamp(),
        }
    }
}
