use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;

/// A newly mined block message.
#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    /// Serialized block data
    pub data: Vec<u8>,
}

impl Block {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Message for Block {
    fn name() -> MessageName {
        MessageName::from("block")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        Ok(Self {
            data: bincode::deserialize(&vec)?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(bincode::serialize(&self.data)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_testing::consensus::BLOCK_1;

    #[test]
    fn test_block() {
        let message = Block::new(BLOCK_1.to_vec());

        let serialized = message.serialize().unwrap();
        let deserialized = Block::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
