#[derive(Debug, Fail)]
pub enum PRFError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "incorrect input length: {}", _0)]
    IncorrectInputLength(usize),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "element is not of prime order")]
    NotPrimeOrder,
}
