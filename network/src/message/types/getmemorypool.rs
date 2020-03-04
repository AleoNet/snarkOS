use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;

#[derive(Debug, PartialEq, Clone)]
pub struct GetMemoryPool;

impl Message for GetMemoryPool {
    fn name() -> MessageName {
        MessageName::from("getmempool")
    }

    fn deserialize(vec: Vec<u8>) -> Result<Self, MessageError> {
        if vec.len() != 0 {
            return Err(MessageError::InvalidLength(vec.len(), 0));
        }

        Ok(Self)
    }

    fn serialize(&self) -> Result<Vec<u8>, MessageError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_memory_pool() {
        let message = GetMemoryPool;

        let serialized = message.serialize().unwrap();
        let deserialized = GetMemoryPool::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
