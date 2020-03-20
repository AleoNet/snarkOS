use std::fmt::Debug;

#[derive(Debug, Fail)]
pub enum AmountError {
    #[fail(display = "the amount: {} exceeds the supply bounds of {}", _0, _1)]
    AmountOutOfBounds(String, String),

    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "invalid amount: {}", _0)]
    InvalidAmount(String),
}
