use snarkvm_errors::algorithms::commitment::CommitmentError;

#[derive(Debug, Error)]
pub enum BindingSignatureError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    CommitmentError(CommitmentError),

    #[error("Affine point is not in the correct subgroup on curve {:?}", _0)]
    NotInCorrectSubgroupOnCurve(Vec<u8>),

    #[error("{}", _0)]
    Message(String),

    #[error("The value balance is out of bounds: {}", _0)]
    OutOfBounds(i64),
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
