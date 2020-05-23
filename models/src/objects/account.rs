use snarkos_errors::objects::AccountError;
use snarkvm_models::algorithms::{CommitmentScheme, SignatureScheme};
use snarkvm_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;

pub trait AccountScheme: FromBytes + ToBytes {
    type AccountPublicKey: Default;
    type AccountPrivateKey: Default;
    type CommitmentScheme: CommitmentScheme;
    type SignatureScheme: SignatureScheme;

    fn new<R: Rng>(
        signature_parameters: &Self::SignatureScheme,
        commitment_parameters: &Self::CommitmentScheme,
        metadata: &[u8; 32],
        rng: &mut R,
    ) -> Result<Self, AccountError>;
}
