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

use crate::curves::FieldError;

#[derive(Debug, Error)]
pub enum GroupError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    FieldError(FieldError),

    #[error("Invalid group element")]
    InvalidGroupElement,

    #[error("Attempting to parse an invalid string into a group element")]
    InvalidString,

    #[error("{}", _0)]
    Message(String),

    #[error("Attempting to parse an empty string into a group element")]
    ParsingEmptyString,

    #[error("Attempting to parse a non-digit character into a group element")]
    ParsingNonDigitCharacter,
}

impl From<FieldError> for GroupError {
    fn from(error: FieldError) -> Self {
        GroupError::FieldError(error)
    }
}

impl From<std::io::Error> for GroupError {
    fn from(error: std::io::Error) -> Self {
        GroupError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<GroupError> for std::io::Error {
    fn from(error: GroupError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{}", error))
    }
}
