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

use crate::{algorithms::SNARKError, curves::constraint_field::ConstraintFieldError, parameters::ParametersError};

use std::io::Error as IoError;
use thiserror::Error;

#[derive(Debug, Error)]
/// An error when generating/verifying a Proof of Succinct Work
pub enum PoswError {
    /// Thrown when the parameters cannot be loaded
    #[error("could not load PoSW parameters: {0}")]
    Parameters(#[from] ParametersError),

    /// Thrown when a proof fails verification
    #[error("could not verify PoSW")]
    PoswVerificationFailed,

    /// Thrown when there's an internal error in the underlying SNARK
    #[error(transparent)]
    SnarkError(#[from] SNARKError),

    /// Thrown when there's an IO error
    #[error(transparent)]
    IoError(#[from] IoError),

    /// Thrown if the mask conversion to a field element fails
    #[error(transparent)]
    ConstraintFieldError(#[from] ConstraintFieldError),
}
