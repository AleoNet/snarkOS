#[derive(Debug, Fail)]
pub enum MessageNameError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "Invalid message name length {}. Expected length of 12", _0)]
    InvalidLength(usize),
}
