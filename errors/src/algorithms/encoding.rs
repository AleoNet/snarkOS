#[derive(Debug, Error)]
pub enum EncodingError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),
}
