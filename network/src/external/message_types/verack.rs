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

use serde::{Deserialize, Serialize};

use std::net::SocketAddr;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/verack.md"))]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Verack {
    /// The random nonce of the connection request.
    pub nonce: u64,
    /// The IP address of the sender.
    pub sender: SocketAddr,
    /// The IP address of the recipient.
    pub receiver: SocketAddr,
}

impl Verack {
    pub fn new(nonce: u64, sender: SocketAddr, receiver: SocketAddr) -> Self {
        Self {
            nonce,
            sender,
            receiver,
        }
    }
}
