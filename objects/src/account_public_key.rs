use crate::{account_format, AccountPrivateKey};
use snarkos_errors::objects::AccountError;
use snarkos_models::{algorithms::CommitmentScheme, dpc::DPCComponents};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use bech32::{Bech32, ToBase32};
use std::{
    fmt,
    io::{Read, Result as IoResult, Write},
};

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: DPCComponents"),
    Clone(bound = "C: DPCComponents"),
    PartialEq(bound = "C: DPCComponents"),
    Eq(bound = "C: DPCComponents")
)]
pub struct AccountPublicKey<C: DPCComponents> {
    pub commitment: <C::AccountCommitment as CommitmentScheme>::Output,
}

impl<C: DPCComponents> AccountPublicKey<C> {
    /// Creates a new account public key from an account private key.
    // TODO: Add testnet account support.
    pub fn from(parameters: &C::AccountCommitment, private_key: &AccountPrivateKey<C>) -> Result<Self, AccountError> {
        // Construct the commitment input for the account public key.
        let commit_input = to_bytes![private_key.pk_sig, private_key.sk_prf, private_key.metadata]?;

        Ok(Self {
            commitment: C::AccountCommitment::commit(parameters, &commit_input, &private_key.r_pk)?,
        })
    }
}

impl<C: DPCComponents> ToBytes for AccountPublicKey<C> {
    // TODO: Add testnet account support.
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.commitment.write(&mut writer)
    }
}

impl<C: DPCComponents> FromBytes for AccountPublicKey<C> {
    /// Reads in an account public key buffer.
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let commitment: <C::AccountCommitment as CommitmentScheme>::Output = FromBytes::read(&mut reader)?;

        Ok(Self { commitment })
    }
}

impl<C: DPCComponents> fmt::Display for AccountPublicKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut public_key = [0u8; 32];
        self.commitment
            .write(&mut public_key[0..32])
            .expect("public key formatting failed");

        let prefix = account_format::PUBLIC_KEY_PREFIX.to_string();

        let result = Bech32::new(prefix, public_key.to_base32());
        result.unwrap().fmt(f)
    }
}

impl<C: DPCComponents> fmt::Debug for AccountPublicKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AccountPublicKey {{ commitment: {:?} }}", self.commitment,)
    }
}
