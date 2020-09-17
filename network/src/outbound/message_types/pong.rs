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
    message::{Message, MessageName},
    message_types::Ping,
};
use snarkos_errors::network::message::MessageError;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/pong.md"))]
#[derive(Debug, PartialEq, Clone)]
pub struct Pong {
    /// Unique ping protocol identifier
    pub nonce: u64,
}

impl Pong {
    pub fn new(ping: Ping) -> Self {
        Self { nonce: ping.nonce }
    }
}

impl Message for Pong {
    fn name() -> MessageName {
        MessageName::from("pong")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        if vec.len() != 8 {
            return Err(MessageError::InvalidLength(vec.len(), 8));
        }

        let mut reader = Cursor::new(vec);

        Ok(Self {
            nonce: reader.read_u64::<BigEndian>().expect("unable to read u64"),
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        let mut writer = vec![];
        writer.write_u64::<BigEndian>(self.nonce)?;

        Ok(writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_pong() {
        let mut rng = rand::thread_rng();
        let message = Pong {
            nonce: rng.gen::<u64>(),
        };

        let serialized = message.serialize().unwrap();
        let deserialized = Pong::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
