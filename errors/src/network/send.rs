use crate::network::message::MessageError;
use crate::network::ConnectError;
use crate::objects::BlockError;

#[derive(Debug, Fail)]
pub enum SendError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    ConnectError(ConnectError),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "{}", _0)]
    MessageError(MessageError),

    #[fail(display = "{}", _0)]
    BlockError(BlockError),
}

impl From<BlockError> for SendError {
    fn from(error: BlockError) -> Self {
        SendError::BlockError(error)
    }
}

impl From<ConnectError> for SendError {
    fn from(error: ConnectError) -> Self {
        SendError::ConnectError(error)
    }
}

impl From<MessageError> for SendError {
    fn from(error: MessageError) -> Self {
        SendError::MessageError(error)
    }
}
