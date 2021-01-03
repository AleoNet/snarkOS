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

use crate::{
    errors::message::MessageError,
    external::message::{Message, MessageName},
};

use chrono::{DateTime, Utc};
use std::net::SocketAddr;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/peers.md"))]
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Peers {
    /// A list of peer addresses and their last seen dates
    pub addresses: Vec<(SocketAddr, DateTime<Utc>)>,
}

impl Peers {
    pub fn new(addresses: Vec<(SocketAddr, DateTime<Utc>)>) -> Self {
        Self { addresses }
    }
}

impl Message for Peers {
    fn name() -> MessageName {
        MessageName::from("peers")
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, MessageError> {
        Ok(Self {
            addresses: bincode::deserialize(bytes)?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(bincode::serialize(&self.addresses)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peers() {
        let message = Peers { addresses: Vec::new() };

        let serialized = message.serialize().unwrap();
        let deserialized = Peers::deserialize(&serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
