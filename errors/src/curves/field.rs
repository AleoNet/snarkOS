#[derive(Debug, Error)]
pub enum FieldError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("Invalid field element")]
    InvalidFieldElement,

    #[error("Attempting to parse an invalid string into a field element")]
    InvalidString,

    #[error("{}", _0)]
    Message(String),

    #[error("Attempting to parse an empty string into a field element")]
    ParsingEmptyString,

    #[error("Attempting to parse a non-digit character into a field element")]
    ParsingNonDigitCharacter,
}

impl From<std::io::Error> for FieldError {
    fn from(error: std::io::Error) -> Self {
        FieldError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<FieldError> for std::io::Error {
    fn from(error: FieldError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{}", error))
    }
}
