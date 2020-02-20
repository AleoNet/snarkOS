use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;

use crate::message::types::Ping;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

#[derive(Debug, PartialEq)]
pub struct Pong {
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
