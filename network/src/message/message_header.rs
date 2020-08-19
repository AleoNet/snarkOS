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

use crate::message::MessageName;
use snarkos_errors::network::message::MessageHeaderError;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

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
        let mut result = vec![];
        result.extend_from_slice(&self.name.as_bytes());

        let mut wtr = vec![];
        wtr.write_u32::<BigEndian>(self.len)?;

        result.extend_from_slice(&wtr);

        Ok(result)
    }

    pub fn deserialize(vec: Vec<u8>) -> Result<Self, MessageHeaderError> {
        if vec.len() != 16 {
            return Err(MessageHeaderError::InvalidLength(vec.len()));
        }

        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&vec[..]);

        Ok(MessageHeader::from(bytes))
    }
}

impl From<[u8; 16]> for MessageHeader {
    fn from(bytes: [u8; 16]) -> Self {
        let mut name_bytes = [0u8; 12];
        name_bytes.copy_from_slice(&bytes[..12]);

        let mut rdr = Cursor::new(bytes[12..].to_vec());

        Self {
            name: MessageName::from(name_bytes),
            len: rdr.read_u32::<BigEndian>().expect("unable to read u32"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_header() {
        let header = MessageHeader {
            name: MessageName::from("ping"),
            len: 4u32,
        };

        assert_eq!(header.serialize().unwrap(), vec![
            112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4
        ]);
    }

    #[test]
    fn deserialize_header() {
        let header = MessageHeader {
            name: MessageName::from("ping"),
            len: 4u32,
        };

        assert_eq!(
            MessageHeader::deserialize(vec![112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4]).unwrap(),
            header
        )
    }

    #[test]
    fn header_from_bytes() {
        let header = MessageHeader {
            name: MessageName::from("ping"),
            len: 4u32,
        };

        assert_eq!(
            header,
            MessageHeader::from([112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4])
        );
    }
}
