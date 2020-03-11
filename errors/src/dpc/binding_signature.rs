use crate::algorithms::commitment::CommitmentError;

#[derive(Debug, Fail)]
pub enum BindingSignatureError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    CommitmentError(CommitmentError),

    #[fail(display = "{}", _0)]
    Message(String),
}

impl From<CommitmentError> for BindingSignatureError {
    fn from(error: CommitmentError) -> Self {
        BindingSignatureError::CommitmentError(error)
    }
}

impl From<std::io::Error> for BindingSignatureError {
    fn from(error: std::io::Error) -> Self {
        BindingSignatureError::Crate("std::io", format!("{:?}", error))
    }
}
