use crate::gadgets::SynthesisError;

#[derive(Debug, Fail)]
pub enum ConstraintFieldError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "{}", _0)]
    SynthesisError(SynthesisError),
}

impl From<SynthesisError> for ConstraintFieldError {
    fn from(error: SynthesisError) -> Self {
        ConstraintFieldError::SynthesisError(error)
    }
}

impl From<std::io::Error> for ConstraintFieldError {
    fn from(error: std::io::Error) -> Self {
        ConstraintFieldError::Crate("std::io", format!("{:?}", error))
    }
}
