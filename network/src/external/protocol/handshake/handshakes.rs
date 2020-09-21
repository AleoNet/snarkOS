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

use crate::external::{Handshake, HandshakeStatus};

use std::{collections::HashMap, net::SocketAddr};

/// Stores the address and latest state of peers we are handshaking with.
#[derive(Clone, Debug, Default)]
pub struct Handshakes {
    handshakes: HashMap<SocketAddr, Handshake>,
}

impl Handshakes {
    /// Construct a new store of connected peer `Handshakes`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the state of the handshake at a peer address.
    pub fn get_state(&self, address: SocketAddr) -> Option<HandshakeStatus> {
        match self.handshakes.get(&address) {
            Some(stored_handshake) => Some(stored_handshake.get_state()),
            None => None,
        }
    }
}
