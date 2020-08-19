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

use crate::algorithms::CRHError;

#[derive(Debug, Error)]
pub enum MerkleError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    CRHError(CRHError),

    #[error("Incorrect leaf index: {}", _0)]
    IncorrectLeafIndex(usize),

    #[error("Incorrect path length: {}", _0)]
    IncorrectPathLength(usize),

    #[error("Invalid leaf")]
    InvalidLeaf,

    #[error("Invalid path length: {}. Must be less than or equal to: {}", _0, _1)]
    InvalidPathLength(usize, usize),

    #[error("Invalid tree depth: {}. Must be less than or equal to: {}", _0, _1)]
    InvalidTreeDepth(usize, usize),

    #[error("{}", _0)]
    Message(String),
}

impl From<CRHError> for MerkleError {
    fn from(error: CRHError) -> Self {
        MerkleError::CRHError(error)
    }
}

impl From<std::io::Error> for MerkleError {
    fn from(error: std::io::Error) -> Self {
        MerkleError::Crate("std::io", format!("{:?}", error))
    }
}
