#[derive(Debug, Fail)]
pub enum ConstraintFieldError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),
}

impl From<std::io::Error> for ConstraintFieldError {
    fn from(error: std::io::Error) -> Self {
        ConstraintFieldError::Crate("std::io", format!("{:?}", error))
    }
}
