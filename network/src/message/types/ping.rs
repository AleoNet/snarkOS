use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use rand::Rng;
use std::io::Cursor;

#[derive(Debug, PartialEq, Clone)]
pub struct Ping {
    pub nonce: u64,
}

impl Ping {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            nonce: rng.gen::<u64>(),
        }
    }
}

impl Message for Ping {
    fn name() -> MessageName {
        MessageName::from("ping")
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

    #[test]
    fn test_ping() {
        let message = Ping::new();

        let serialized = message.serialize().unwrap();
        let deserialized = Ping::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
