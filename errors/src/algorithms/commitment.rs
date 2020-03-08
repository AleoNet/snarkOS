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
