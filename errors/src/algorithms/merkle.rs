use crate::algorithms::CRHError;

#[derive(Debug, Fail)]
pub enum MerkleError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    CRHError(CRHError),

    #[fail(display = "Incorrect leaf index: {}", _0)]
    IncorrectLeafIndex(usize),

    #[fail(display = "Incorrect path length: {}", _0)]
    IncorrectPathLength(usize),

    #[fail(display = "{}", _0)]
    Message(String),
}

impl From<CRHError> for MerkleError {
    fn from(error: CRHError) -> Self {
        MerkleError::CRHError(error)
    }
}

impl From<std::io::Error> for MerkleError {
    fn from(error: std::io::Error) -> Self {
        MerkleError::Crate("std::io", format!("{:?}", error))
    }
}
