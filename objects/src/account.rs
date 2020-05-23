use crate::{AccountPrivateKey, AccountPublicKey};
use snarkos_errors::objects::AccountError;
use snarkos_models::{dpc::DPCComponents, objects::AccountScheme};
use snarkvm_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;
use std::io::{Read, Result as IoResult, Write};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DPCComponents"))]
pub struct Account<C: DPCComponents> {
    pub public_key: AccountPublicKey<C>,
    pub private_key: AccountPrivateKey<C>,
}

impl<C: DPCComponents> AccountScheme for Account<C> {
    type AccountPrivateKey = AccountPrivateKey<C>;
    type AccountPublicKey = AccountPublicKey<C>;
    type CommitmentScheme = C::AccountCommitment;
    type SignatureScheme = C::Signature;

    /// Creates a new account.
    fn new<R: Rng>(
        signature_parameters: &Self::SignatureScheme,
        commitment_parameters: &Self::CommitmentScheme,
        metadata: &[u8; 32],
        rng: &mut R,
    ) -> Result<Self, AccountError> {
        let private_key = AccountPrivateKey::new(signature_parameters, metadata, rng)?;
        let public_key = AccountPublicKey::from(commitment_parameters, &private_key)?;

        Ok(Self {
            private_key,
            public_key,
        })
    }
}

impl<C: DPCComponents> ToBytes for Account<C> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.public_key.write(&mut writer)?;
        self.private_key.write(&mut writer)
    }
}

impl<C: DPCComponents> FromBytes for Account<C> {
    /// Reads in an account buffer.
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let public_key: AccountPublicKey<C> = FromBytes::read(&mut reader)?;
        let private_key: AccountPrivateKey<C> = FromBytes::read(&mut reader)?;

        Ok(Self {
            private_key,
            public_key,
        })
    }
}
