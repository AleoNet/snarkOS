use crate::{algorithms::SNARKError, curves::constraint_field::ConstraintFieldError, parameters::ParametersError};

use thiserror::Error;
use std::io::Error as IoError;

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
