use crate::{
    network::{message::MessageError, ConnectError},
    objects::BlockError,
};

#[derive(Debug, Error)]
pub enum SendError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    ConnectError(ConnectError),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    MessageError(MessageError),

    #[error("{}", _0)]
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
