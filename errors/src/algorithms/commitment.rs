use crate::algorithms::CRHError;

#[derive(Debug, Fail)]
pub enum CommitmentError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    CRHError(CRHError),

    #[fail(display = "{}", _0)]
    Message(String),
}

impl From<CRHError> for CommitmentError {
    fn from(error: CRHError) -> Self {
        CommitmentError::CRHError(error)
    }
}

impl From<std::io::Error> for CommitmentError {
    fn from(error: std::io::Error) -> Self {
        CommitmentError::Crate("std::io", format!("{:?}", error))
    }
}
