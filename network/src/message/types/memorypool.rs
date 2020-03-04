use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;

#[derive(Debug, PartialEq, Clone)]
pub struct MemoryPool {
    pub transactions: Vec<Vec<u8>>,
}

impl MemoryPool {
    pub fn new(transactions: Vec<Vec<u8>>) -> Self {
        Self { transactions }
    }
}

impl Message for MemoryPool {
    fn name() -> MessageName {
        MessageName::from("memorypool")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        Ok(Self {
            transactions: bincode::deserialize(&vec)?,
        })
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(bincode::serialize(&self.transactions)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkos_consensus::test_data::TRANSACTION;

    #[test]
    fn test_memory_pool() {
        let message = MemoryPool::new(vec![hex::decode(TRANSACTION).unwrap()]);

        let serialized = message.serialize().unwrap();
        let deserialized = MemoryPool::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
