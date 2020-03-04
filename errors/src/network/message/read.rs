#[derive(Debug, Fail)]
pub enum StreamReadError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),
}

impl From<std::io::Error> for StreamReadError {
    fn from(error: std::io::Error) -> Self {
        StreamReadError::Crate("std::io", format!("{:?}", error))
    }
}
