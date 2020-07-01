use crate::algorithms::{CommitmentScheme, SignatureScheme};
use snarkos_errors::objects::AccountError;

use rand::Rng;

pub trait AccountScheme: Sized {
    type AccountPublicKey: Default;
    type AccountPrivateKey: Default;
    type CommitmentScheme: CommitmentScheme;
    type SignatureScheme: SignatureScheme;

    fn new<R: Rng>(
        signature_parameters: &Self::SignatureScheme,
        commitment_parameters: &Self::CommitmentScheme,
        rng: &mut R,
    ) -> Result<Self, AccountError>;
}
