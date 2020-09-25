// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

#[derive(Debug, Error)]
pub enum PrivateKeyError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),
}

impl From<hex::FromHexError> for PrivateKeyError {
    fn from(error: hex::FromHexError) -> Self {
        PrivateKeyError::Crate("hex", format!("{:?}", error))
    }
}

impl From<snarkos_errors::objects::account::AccountError> for PrivateKeyError {
    fn from(error: snarkos_errors::objects::account::AccountError) -> Self {
        PrivateKeyError::Crate("snarkos_errors::objects::account", format!("{:?}", error))
    }
}

impl From<snarkos_errors::algorithms::signature::SignatureError> for PrivateKeyError {
    fn from(error: snarkos_errors::algorithms::signature::SignatureError) -> Self {
        PrivateKeyError::Crate("snarkos_errors::algorithms::signature", format!("{:?}", error))
    }
}

impl From<std::io::Error> for PrivateKeyError {
    fn from(error: std::io::Error) -> Self {
        PrivateKeyError::Crate("std::io", format!("{:?}", error))
    }
}

#[derive(Debug, Error)]
pub enum ViewKeyError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),
}

impl From<hex::FromHexError> for ViewKeyError {
    fn from(error: hex::FromHexError) -> Self {
        ViewKeyError::Crate("hex", format!("{:?}", error))
    }
}

impl From<snarkos_errors::algorithms::signature::SignatureError> for ViewKeyError {
    fn from(error: snarkos_errors::algorithms::signature::SignatureError) -> Self {
        ViewKeyError::Crate("snarkos_errors::algorithms::signature", format!("{:?}", error))
    }
}

impl From<snarkos_errors::objects::account::AccountError> for ViewKeyError {
    fn from(error: snarkos_errors::objects::account::AccountError) -> Self {
        ViewKeyError::Crate("snarkos_errors::objects::account", format!("{:?}", error))
    }
}

impl From<std::io::Error> for ViewKeyError {
    fn from(error: std::io::Error) -> Self {
        ViewKeyError::Crate("std::io", format!("{:?}", error))
    }
}

#[derive(Debug, Error)]
pub enum AddressError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),
}

impl From<snarkos_errors::algorithms::signature::SignatureError> for AddressError {
    fn from(error: snarkos_errors::algorithms::signature::SignatureError) -> Self {
        AddressError::Crate("snarkos_errors::algorithms::signature", format!("{:?}", error))
    }
}

impl From<snarkos_errors::objects::account::AccountError> for AddressError {
    fn from(error: snarkos_errors::objects::account::AccountError) -> Self {
        AddressError::Crate("snarkos_errors::objects::account", format!("{:?}", error))
    }
}

impl From<std::io::Error> for AddressError {
    fn from(error: std::io::Error) -> Self {
        AddressError::Crate("std::io", format!("{:?}", error))
    }
}
