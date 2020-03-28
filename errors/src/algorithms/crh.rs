#[derive(Debug, Fail)]
pub enum CRHError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),
}

impl From<std::io::Error> for CRHError {
    fn from(error: std::io::Error) -> Self {
        CRHError::Crate("std::io", format!("{:?}", error))
    }
}
