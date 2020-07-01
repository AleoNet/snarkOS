use crate::{AccountPrivateKey, AccountPublicKey};
use snarkos_errors::objects::AccountError;
use snarkos_models::{dpc::DPCComponents, objects::AccountScheme};

use rand::Rng;
use std::fmt;

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DPCComponents"))]
pub struct Account<C: DPCComponents> {
    pub private_key: AccountPrivateKey<C>,
    pub public_key: AccountPublicKey<C>,
}

impl<C: DPCComponents> AccountScheme for Account<C> {
    type AccountPrivateKey = AccountPrivateKey<C>;
    type AccountPublicKey = AccountPublicKey<C>;
    type CommitmentScheme = C::AccountCommitment;
    type SignatureScheme = C::AccountSignature;

    /// Creates a new account.
    fn new<R: Rng>(
        signature_parameters: &Self::SignatureScheme,
        commitment_parameters: &Self::CommitmentScheme,
        metadata: &[u8; 32],
        rng: &mut R,
    ) -> Result<Self, AccountError> {
        let private_key = AccountPrivateKey::new(signature_parameters, commitment_parameters, metadata, rng)?;
        let public_key = AccountPublicKey::from(signature_parameters, commitment_parameters, &private_key)?;

        Ok(Self {
            private_key,
            public_key,
        })
    }
}

impl<C: DPCComponents> fmt::Display for Account<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Account {{ private_key: {}, public_key: {} }}",
            self.private_key, self.public_key,
        )
    }
}

impl<C: DPCComponents> fmt::Debug for Account<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Account {{ private_key: {:?}, public_key: {:?} }}",
            self.private_key, self.public_key,
        )
    }
}
