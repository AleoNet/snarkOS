use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;
use snarkos_objects::BlockHeaderHash;

/// Vector of block hashes from a sync node
#[derive(Debug, PartialEq, Clone)]
pub struct Sync {
    /// hashes of blocks to share
    pub block_hashes: Vec<BlockHeaderHash>,
}

impl Sync {
    pub fn new(block_hashes: Vec<BlockHeaderHash>) -> Self {
        Self { block_hashes }
    }
}

impl Message for Sync {
    fn name() -> MessageName {
        MessageName::from("sync")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        Ok(Self {
            block_hashes: bincode::deserialize(&vec)?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(bincode::serialize(&self.block_hashes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_consensus::test_data::BLOCK_1_HEADER_HASH;

    #[test]
    fn test_sync() {
        let data = BlockHeaderHash::new(hex::decode(BLOCK_1_HEADER_HASH).unwrap());
        let message = Sync::new(vec![data]);

        let serialized = message.serialize().unwrap();
        let deserialized = Sync::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
