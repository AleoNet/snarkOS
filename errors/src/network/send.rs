use crate::network::PeerError;

#[derive(Debug, Fail)]
pub enum SendError {
    #[fail(display = "{}", _0)]
    PeerError(PeerError),

    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),
}

impl From<PeerError> for SendError {
    fn from(error: PeerError) -> Self {
        SendError::PeerError(error)
    }
}
