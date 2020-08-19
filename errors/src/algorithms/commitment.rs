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

use crate::algorithms::CRHError;

#[derive(Debug, Error)]
pub enum CommitmentError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("incorrect input length {} for window params {}x{}", _0, _1, _2)]
    IncorrectInputLength(usize, usize, usize),

    #[error("{}", _0)]
    CRHError(CRHError),

    #[error("{}", _0)]
    Message(String),
}

impl From<CRHError> for CommitmentError {
    fn from(error: CRHError) -> Self {
        CommitmentError::CRHError(error)
    }
}

impl From<Error> for CommitmentError {
    fn from(error: Error) -> Self {
        CommitmentError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<CommitmentError> for Error {
    fn from(error: CommitmentError) -> Error {
        Error::new(ErrorKind::Other, error.to_string())
    }
}
