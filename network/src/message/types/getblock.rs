use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;
use snarkos_objects::BlockHeaderHash;

/// A request for a block with the specified hash.
/// See network/protocol/sync.rs for more details.
#[derive(Debug, PartialEq, Clone)]
pub struct GetBlock {
    /// Header hash of requested block
    pub block_hash: BlockHeaderHash,
}

impl GetBlock {
    pub fn new(block_hash: BlockHeaderHash) -> Self {
        Self { block_hash }
    }
}

impl Message for GetBlock {
    fn name() -> MessageName {
        MessageName::from("getblock")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        Ok(Self {
            block_hash: bincode::deserialize(&vec)?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(bincode::serialize(&self.block_hash)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_consensus::test_data::BLOCK_1_HEADER_HASH;

    #[test]
    fn test_block() {
        let block_hash = BlockHeaderHash::new(hex::decode(BLOCK_1_HEADER_HASH).unwrap());
        let message = GetBlock::new(block_hash);

        let serialized = message.serialize().unwrap();
        let deserialized = GetBlock::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
