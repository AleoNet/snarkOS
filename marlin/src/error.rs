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

use crate::ahp::Error as AHPError;

/// A `enum` specifying the possible failure modes of the `SNARK`.
#[derive(Debug)]
pub enum Error<E> {
    /// The index is too large for the universal public parameters.
    IndexTooLarge,
    /// There was an error in the underlying holographic IOP.
    AHPError(AHPError),
    /// There was an error in the underlying polynomial commitment.
    PolynomialCommitmentError(E),
}

impl<E> From<AHPError> for Error<E> {
    fn from(err: AHPError) -> Self {
        Error::AHPError(err)
    }
}

impl<E> Error<E> {
    /// Convert an error in the underlying polynomial commitment scheme
    /// to a `Error`.
    pub fn from_pc_err(err: E) -> Self {
        Error::PolynomialCommitmentError(err)
    }
}
