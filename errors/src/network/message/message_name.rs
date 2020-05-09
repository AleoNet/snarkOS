#[derive(Debug, Error)]
pub enum MessageNameError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("Invalid message name length {}. Expected length of 12", _0)]
    InvalidLength(usize),
}
