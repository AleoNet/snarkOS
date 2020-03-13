use crate::message::MessageName;
use snarkos_errors::network::message::MessageHeaderError;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

/// A fixed size message corresponding to a variable sized message.
#[derive(Debug, PartialEq, Eq)]
pub struct MessageHeader {
    pub magic: u32,
    pub len: u32,
    pub name: MessageName,
}

impl MessageHeader {
    pub fn new(magic: u32, len: u32, name: MessageName) -> Self {
        MessageHeader { magic, len, name }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, MessageHeaderError> {
        let mut result = vec![];

        result.write_u32::<BigEndian>(self.magic)?;
        result.write_u32::<BigEndian>(self.len)?;
        result.extend_from_slice(&self.name.as_bytes());

        Ok(result)
    }

    pub fn deserialize(vec: Vec<u8>) -> Result<Self, MessageHeaderError> {
        if vec.len() != 20 {
            return Err(MessageHeaderError::InvalidLength(vec.len()));
        }

        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(&vec[..]);

        Ok(MessageHeader::from(bytes))
    }
}

impl From<[u8; 20]> for MessageHeader {
    fn from(bytes: [u8; 20]) -> Self {
        let mut magic_reader = Cursor::new(bytes[0..4].to_vec());
        let mut len_reader = Cursor::new(bytes[4..8].to_vec());

        let mut name_bytes = [0u8; 12];
        name_bytes.copy_from_slice(&bytes[8..]);

        Self {
            magic: magic_reader.read_u32::<BigEndian>().expect("unable to read u32"),
            len: len_reader.read_u32::<BigEndian>().expect("unable to read u32"),
            name: MessageName::from(name_bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::MAGIC_MAINNET;

    use super::*;

    #[test]
    fn serialize_header() {
        let header = MessageHeader {
            magic: MAGIC_MAINNET,
            name: MessageName::from("ping"),
            len: 4u32,
        };

        assert_eq!(header.serialize().unwrap(), vec![
            217, 180, 190, 249, 0, 0, 0, 4, 112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0
        ]);
    }

    #[test]
    fn deserialize_header() {
        let header = MessageHeader {
            magic: MAGIC_MAINNET,
            name: MessageName::from("ping"),
            len: 4u32,
        };

        assert_eq!(
            MessageHeader::deserialize(vec![
                217, 180, 190, 249, 0, 0, 0, 4, 112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0
            ])
            .unwrap(),
            header
        )
    }

    #[test]
    fn header_from_bytes() {
        let header = MessageHeader {
            magic: MAGIC_MAINNET,
            name: MessageName::from("ping"),
            len: 4u32,
        };

        assert_eq!(
            header,
            MessageHeader::from([
                217, 180, 190, 249, 0, 0, 0, 4, 112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0
            ])
        );
    }
}
