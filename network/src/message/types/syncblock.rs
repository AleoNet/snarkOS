use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;

/// A serialized block from a sync node
#[derive(Debug, PartialEq, Clone)]
pub struct SyncBlock {
    /// block data
    pub data: Vec<u8>,
}

impl SyncBlock {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Message for SyncBlock {
    fn name() -> MessageName {
        MessageName::from("syncblock")
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
    use snarkos_consensus::test_data::BLOCK_1;

    #[test]
    fn test_sync_block() {
        let data = hex::decode(BLOCK_1).unwrap();
        let message = SyncBlock::new(data);

        let serialized = message.serialize().unwrap();
        let deserialized = SyncBlock::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
