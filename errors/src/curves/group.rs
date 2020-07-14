use crate::curves::FieldError;

#[derive(Debug, Error)]
pub enum GroupError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    FieldError(FieldError),

    #[error("Invalid group element")]
    InvalidGroupElement,

    #[error("Attempting to parse an invalid string into a group element")]
    InvalidString,

    #[error("{}", _0)]
    Message(String),

    #[error("Attempting to parse an empty string into a group element")]
    ParsingEmptyString,

    #[error("Attempting to parse a non-digit character into a group element")]
    ParsingNonDigitCharacter,
}

impl From<FieldError> for GroupError {
    fn from(error: FieldError) -> Self {
        GroupError::FieldError(error)
    }
}

impl From<std::io::Error> for GroupError {
    fn from(error: std::io::Error) -> Self {
        GroupError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<GroupError> for std::io::Error {
    fn from(error: GroupError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{}", error))
    }
}
