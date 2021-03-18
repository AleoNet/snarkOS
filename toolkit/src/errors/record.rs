// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use crate::errors::AddressError;

#[derive(Debug, Error)]
pub enum RecordError {
    #[error("{}", _0)]
    AddressError(AddressError),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("invalid private key for the record")]
    InvalidPrivateKey,
}

impl From<AddressError> for RecordError {
    fn from(error: AddressError) -> Self {
        RecordError::AddressError(error)
    }
}

impl From<hex::FromHexError> for RecordError {
    fn from(error: hex::FromHexError) -> Self {
        RecordError::Crate("hex", format!("{:?}", error))
    }
}

impl From<snarkvm_errors::dpc::DPCError> for RecordError {
    fn from(error: snarkvm_errors::dpc::DPCError) -> Self {
        RecordError::Crate("snarkvm_errors::dpc", format!("{:?}", error))
    }
}

impl From<std::io::Error> for RecordError {
    fn from(error: std::io::Error) -> Self {
        RecordError::Crate("std::io", format!("{:?}", error))
    }
}
