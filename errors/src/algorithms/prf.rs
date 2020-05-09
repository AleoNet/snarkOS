#[derive(Debug, Error)]
pub enum PRFError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("incorrect input length: {}", _0)]
    IncorrectInputLength(usize),

    #[error("{}", _0)]
    Message(String),

    #[error("element is not of prime order")]
    NotPrimeOrder,
}
