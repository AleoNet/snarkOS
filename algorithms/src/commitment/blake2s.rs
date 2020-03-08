use snarkos_errors::algorithms::CommitmentError;
use snarkos_models::algorithms::CommitmentScheme;

use blake2::Blake2s as b2s;
use digest::Digest;
use rand::Rng;

pub struct Blake2sCommitment;

impl CommitmentScheme for Blake2sCommitment {
    type Output = [u8; 32];
    type Parameters = ();
    type Randomness = [u8; 32];

    fn setup<R: Rng>(_: &mut R) -> Self {
        Self
    }

    fn commit(&self, input: &[u8], randomness: &Self::Randomness) -> Result<Self::Output, CommitmentError> {
        let mut h = b2s::new();
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
