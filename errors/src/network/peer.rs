#[derive(Debug, Fail)]
pub enum PeerError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),
}

impl From<bincode::Error> for PeerError {
    fn from(error: bincode::Error) -> Self {
        PeerError::Crate("bincode", format!("{:?}", error))
    }
}

impl From<std::io::Error> for PeerError {
    fn from(error: std::io::Error) -> Self {
        PeerError::Crate("std::io", format!("{:?}", error))
    }
}
