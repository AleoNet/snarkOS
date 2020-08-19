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

use std::io::{Error, ErrorKind};

#[derive(Debug, Error)]
pub enum EncodingError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("Input must be nonzero")]
    InputMustBeNonzero,

    #[error("Invalid group element")]
    InvalidGroupElement,

    #[error("{}", _0)]
    Message(String),
}

impl From<Error> for EncodingError {
    fn from(error: Error) -> Self {
        EncodingError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<EncodingError> for Error {
    fn from(error: EncodingError) -> Error {
        Error::new(ErrorKind::Other, error.to_string())
    }
}
