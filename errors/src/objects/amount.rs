use std::fmt::Debug;

#[derive(Debug, Error)]
pub enum AmountError {
    #[error("the amount: {} exceeds the supply bounds of {}", _0, _1)]
    AmountOutOfBounds(String, String),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("invalid amount: {}", _0)]
    InvalidAmount(String),
}
