use crate::message::MessageName;
use snarkos_errors::network::message::MessageError;

pub trait Message: Send + 'static {
    fn name() -> MessageName;
    fn deserialize(bytes: Vec<u8>) -> Result<Self, MessageError>
    where
        Self: Sized;
    fn serialize(&self) -> Result<Vec<u8>, MessageError>;
}
