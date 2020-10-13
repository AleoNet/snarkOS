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

use crate::external::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;

use chrono::Utc;
use rand::Rng;
use std::net::SocketAddr;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/version.md"))]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Version {
    /// The version number of the sender's node server.
    pub version: u64,
    /// The block height of the sender's node server.
    pub height: u32,
    /// The random nonce of the handshake request.
    pub nonce: u64,
    /// The IP address of the sender.
    pub sender: SocketAddr,
    /// The IP address of the recipient.
    pub receiver: SocketAddr,
    /// The timestamp of this message.
    pub timestamp: i64,
}

impl Version {
    pub fn new(version: u64, height: u32, nonce: u64, sender: SocketAddr, receiver: SocketAddr) -> Self {
        Self {
            version,
            height,
            nonce,
            sender,
            receiver,
            timestamp: Utc::now().timestamp(),
        }
    }

    #[deprecated]
    pub fn new_with_rng(version: u64, height: u32, sender: SocketAddr, receiver: SocketAddr) -> Self {
        let mut rng = rand::thread_rng();

        Self {
            version,
            height,
            nonce: rng.gen::<u64>(),
            sender,
            receiver,
            timestamp: Utc::now().timestamp(),
        }
    }
}

impl Message for Version {
    fn name() -> MessageName {
        MessageName::from("version")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        if vec.len() != 48 {
            return Err(MessageError::InvalidLength(vec.len(), 48));
        }

        Ok(Version {
            version: bincode::deserialize(&vec[..8])?,
            height: bincode::deserialize(&vec[8..12])?,
            nonce: bincode::deserialize(&vec[12..20])?,
            timestamp: bincode::deserialize(&vec[20..28])?,
            receiver: bincode::deserialize(&vec[28..38])?,
            sender: bincode::deserialize(&vec[38..48])?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        let mut writer = vec![];
        writer.extend_from_slice(&bincode::serialize(&self.version)?);
        writer.extend_from_slice(&bincode::serialize(&self.height)?);
        writer.extend_from_slice(&bincode::serialize(&self.nonce)?);
        writer.extend_from_slice(&bincode::serialize(&self.timestamp)?);
        writer.extend_from_slice(&bincode::serialize(&self.receiver)?);
        writer.extend_from_slice(&bincode::serialize(&self.sender)?);
        Ok(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let version = Version::new_with_rng(
            1u64,
            1u32,
            "127.0.0.1:4130".parse::<SocketAddr>().unwrap(),
            "127.0.0.1:4130".parse::<SocketAddr>().unwrap(),
        );

        let serialized = version.serialize().unwrap();

        let deserialized = Version::deserialize(serialized).unwrap();

        assert_eq!(version, deserialized);
    }
}
