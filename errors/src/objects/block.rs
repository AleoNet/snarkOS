use crate::objects::TransactionError;

#[derive(Debug, Fail)]
pub enum BlockError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "{}", _0)]
    TransactionError(TransactionError),
}

impl From<TransactionError> for BlockError {
    fn from(error: TransactionError) -> Self {
        BlockError::TransactionError(error)
    }
}

impl From<BlockError> for Box<dyn std::error::Error> {
    fn from(error: BlockError) -> Self {
        error.into()
    }
}
