use crate::algorithms::{CommitmentScheme, EncryptionScheme, SignatureScheme};
use snarkos_errors::objects::AccountError;

use rand::Rng;

pub trait AccountScheme: Sized {
    type AccountPublicKey: Default;
    type AccountPrivateKey;
    type CommitmentScheme: CommitmentScheme;
    type EncryptionScheme: EncryptionScheme;
    type SignatureScheme: SignatureScheme;

    fn new<R: Rng>(
        signature_parameters: &Self::SignatureScheme,
        commitment_parameters: &Self::CommitmentScheme,
        encryption_parameters: &Self::EncryptionScheme,
        rng: &mut R,
    ) -> Result<Self, AccountError>;
}
