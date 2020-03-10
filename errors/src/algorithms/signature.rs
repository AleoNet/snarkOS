#[derive(Debug, Fail)]
pub enum SignatureError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),
}

impl From<std::io::Error> for SignatureError {
    fn from(error: std::io::Error) -> Self {
        SignatureError::Crate("std::io", format!("{:?}", error))
    }
}
