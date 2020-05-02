use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;
use snarkos_objects::BlockHeaderHash;

/// A request for knowledge of specified block locator hashes.
/// See network/protocol/sync.rs for more details.
#[derive(Debug, PartialEq, Clone)]
pub struct GetSync {
    /// hashes of blocks requested
    pub block_locator_hashes: Vec<BlockHeaderHash>,
}

impl GetSync {
    pub fn new(block_locator_hashes: Vec<BlockHeaderHash>) -> Self {
        Self { block_locator_hashes }
    }
}

impl Message for GetSync {
    fn name() -> MessageName {
        MessageName::from("getsync")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        Ok(Self {
            block_locator_hashes: bincode::deserialize(&vec)?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(bincode::serialize(&self.block_locator_hashes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_consensus::test_data::BLOCK_1_HEADER_HASH;

    #[test]
    fn test_get_sync() {
        let data = BlockHeaderHash::new(BLOCK_1_HEADER_HASH.to_vec());
        let message = GetSync::new(vec![data]);

        let serialized = message.serialize().unwrap();
        let deserialized = GetSync::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
