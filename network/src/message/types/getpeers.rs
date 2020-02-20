use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;

#[derive(Debug, PartialEq)]
pub struct GetPeers;

impl Message for GetPeers {
    fn name() -> MessageName {
        MessageName::from("getpeers")
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
    fn test_getpeers() {
        let message = GetPeers;

        let serialized = message.serialize().unwrap();
        let deserialized = GetPeers::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
