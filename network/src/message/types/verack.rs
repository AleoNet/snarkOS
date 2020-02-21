use crate::message::{Message, MessageName};
use snarkos_errors::network::message::MessageError;

#[derive(Debug, PartialEq, Clone)]
pub struct Verack;

impl Message for Verack {
    fn name() -> MessageName {
        MessageName::from("verack")
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
    fn test_verack() {
        let message = Verack;

        let serialized = message.serialize().unwrap();
        let deserialized = Verack::deserialize(serialized).unwrap();

        assert_eq!(message, deserialized);
    }
}
