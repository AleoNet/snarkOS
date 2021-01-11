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

use crate::{errors::message::MessageHeaderError, external::message::MessageName};

use byteorder::{BigEndian, WriteBytesExt};
use std::convert::TryFrom;

/// A fixed size message corresponding to a variable sized message.
#[derive(Debug, PartialEq, Eq)]
pub struct MessageHeader {
    pub name: MessageName,
    pub len: u32,
}

impl MessageHeader {
    pub fn new(name: MessageName, len: u32) -> Self {
        MessageHeader { name, len }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, MessageHeaderError> {
        let mut result = Vec::with_capacity(5);
        result.push(self.name as u8);
        result.write_u32::<BigEndian>(self.len)?;

        Ok(result)
    }

    pub fn deserialize(vec: Vec<u8>) -> Result<Self, MessageHeaderError> {
        if vec.len() != 5 {
            return Err(MessageHeaderError::InvalidLength(vec.len()));
        }

        let mut bytes = [0u8; 5];
        bytes.copy_from_slice(&vec[..]);

        Ok(MessageHeader::from(bytes))
    }
}

// FIXME(ljedrz): use TryFrom instead
impl From<[u8; 5]> for MessageHeader {
    fn from(bytes: [u8; 5]) -> Self {
        let name = MessageName::try_from(bytes[0]).expect("invalid MessageHeader!");

        let mut len = [0u8; 4];
        len.copy_from_slice(&bytes[1..]);
        let len = u32::from_be_bytes(len);

        Self { name, len }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_header() {
        let header = MessageHeader {
            name: MessageName::Block,
            len: 4u32,
        };

        assert_eq!(header.serialize().unwrap(), vec![0, 0, 0, 0, 4]);
    }

    #[test]
    fn deserialize_header() {
        let header = MessageHeader {
            name: MessageName::Block,
            len: 4u32,
        };

        assert_eq!(MessageHeader::deserialize(vec![0, 0, 0, 0, 4]).unwrap(), header)
    }

    #[test]
    fn header_from_bytes() {
        let header = MessageHeader {
            name: MessageName::Block,
            len: 4u32,
        };

        assert_eq!(header, MessageHeader::from([0, 0, 0, 0, 4]));
    }
}
