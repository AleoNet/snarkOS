use crate::{curves::ConstraintFieldError, gadgets::SynthesisError};

#[derive(Debug, Error)]
pub enum SNARKError {
    #[error("{}", _0)]
    ConstraintFieldError(ConstraintFieldError),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    SynthesisError(SynthesisError),
}

impl From<ConstraintFieldError> for SNARKError {
    fn from(error: ConstraintFieldError) -> Self {
        SNARKError::ConstraintFieldError(error)
    }
}

impl From<SynthesisError> for SNARKError {
    fn from(error: SynthesisError) -> Self {
        SNARKError::SynthesisError(error)
    }
}
