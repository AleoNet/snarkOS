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

//impl From<&'static str> for AddressError {
//    fn from(msg: &'static str) -> Self {
//        AddressError::Message(msg.into())
//    }
//}
//
//impl From<PrivateKeyError> for AddressError {
//    fn from(error: PrivateKeyError) -> Self {
//        AddressError::PrivateKeyError(error)
//    }
//}
//
//impl From<PublicKeyError> for AddressError {
//    fn from(error: PublicKeyError) -> Self {
//        AddressError::PublicKeyError(error)
//    }
//}
//
//impl From<base58::FromBase58Error> for AddressError {
//    fn from(error: base58::FromBase58Error) -> Self {
//        AddressError::Crate("base58", format!("{:?}", error))
//    }
//}
//
//impl From<base58_monero::base58::Error> for AddressError {
//    fn from(error: base58_monero::base58::Error) -> Self {
//        AddressError::Crate("base58_monero", format!("{:?}", error))
//    }
//}
//
//impl From<bech32::Error> for AddressError {
//    fn from(error: bech32::Error) -> Self {
//        AddressError::Crate("bech32", format!("{:?}", error))
//    }
//}
//
//impl From<hex::FromHexError> for AddressError {
//    fn from(error: hex::FromHexError) -> Self {
//        AddressError::Crate("hex", format!("{:?}", error))
//    }
//}
//
//impl From<rand_core::Error> for AddressError {
//    fn from(error: rand_core::Error) -> Self {
//        AddressError::Crate("rand", format!("{:?}", error))
//    }
//}
//
//impl From<std::io::Error> for AddressError {
//    fn from(error: std::io::Error) -> Self {
//        AddressError::Crate("std::io", format!("{:?}", error))
//    }
//}
//
//impl From<std::str::Utf8Error> for AddressError {
//    fn from(error: std::str::Utf8Error) -> Self {
//        AddressError::Crate("std::str", format!("{:?}", error))
//    }
//}
//
//impl From<std::string::FromUtf8Error> for AddressError {
//    fn from(error: std::string::FromUtf8Error) -> Self {
//        AddressError::Crate("std::string", format!("{:?}", error))
//    }
//}
