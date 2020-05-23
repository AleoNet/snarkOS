use snarkos_models::storage::Storage;
use snarkvm_errors::algorithms::CommitmentError;
use snarkvm_models::algorithms::CommitmentScheme;
use snarkvm_utilities::bytes::{FromBytes, ToBytes};

use blake2::Blake2s as blake2s;
use digest::Digest;
use rand::Rng;
use std::{
    io::{Read, Result as IoResult, Write},
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Blake2sCommitment;

impl CommitmentScheme for Blake2sCommitment {
    type Output = [u8; 32];
    type Parameters = ();
    type Randomness = [u8; 32];

    fn setup<R: Rng>(_: &mut R) -> Self {
        Self
    }

    fn commit(&self, input: &[u8], randomness: &Self::Randomness) -> Result<Self::Output, CommitmentError> {
        let mut h = blake2s::new();
        h.input(input);
        h.input(randomness.as_ref());

        let mut result = [0u8; 32];
        result.copy_from_slice(&h.result());
        Ok(result)
    }

    fn parameters(&self) -> &Self::Parameters {
        &()
    }
}

/// TODO Remove required placeholder trait impls
///   Blake2sCommitment is different in that it's a CommitmentScheme, with no parameters to store/load.

impl Storage for Blake2sCommitment {
    fn store(&self, _path: &PathBuf) -> IoResult<()> {
        Ok(())
    }

    fn load(_path: &PathBuf) -> IoResult<Self> {
        Ok(Self)
    }
}

impl ToBytes for Blake2sCommitment {
    #[inline]
    fn write<W: Write>(&self, _writer: W) -> IoResult<()> {
        Ok(())
    }
}

impl FromBytes for Blake2sCommitment {
    #[inline]
    fn read<R: Read>(_reader: R) -> IoResult<Self> {
        Ok(Self)
    }
}
