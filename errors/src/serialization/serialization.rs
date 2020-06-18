use std::io;

#[derive(Error, Debug)]
pub enum SerializationError {
    /// During serialization, we didn't have enough space to write extra info.
    #[error("the last byte does not have enough space to encode the extra info bits")]
    NotEnoughSpace,
    /// During serialization, the data was invalid.
    #[error("the input buffer contained invalid data")]
    InvalidData,
    /// During serialization, non-empty flags were given where none were
    /// expected.
    #[error("the call expects empty flags")]
    UnexpectedFlags,
    /// During serialization, we countered an I/O error.
    #[error("IoError: {0}")]
    IoError(#[from] io::Error),
}
