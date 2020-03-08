use crate::{curves::ConstraintFieldError, gadgets::SynthesisError};

#[derive(Debug, Fail)]
pub enum SNARKError {
    #[fail(display = "{}", _0)]
    ConstraintFieldError(ConstraintFieldError),

    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "{}", _0)]
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
