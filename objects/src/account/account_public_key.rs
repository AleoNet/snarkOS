use crate::{account_format, AccountPrivateKey};
use snarkos_errors::objects::AccountError;
use snarkos_models::{algorithms::CommitmentScheme, dpc::DPCComponents};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use bech32::{Bech32, FromBase32, ToBase32};
use std::{
    fmt,
    io::{Read, Result as IoResult, Write},
    str::FromStr,
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
    pub fn from(
        commitment_parameters: &C::AccountCommitment,
        signature_parameters: &C::AccountSignature,
        private_key: &AccountPrivateKey<C>,
    ) -> Result<Self, AccountError> {
        // Construct the commitment input for the account public key.
        let commit_input = to_bytes![
            private_key.pk_sig(signature_parameters)?,
            private_key.sk_prf,
            private_key.metadata
        ]?;

        Ok(Self {
            commitment: C::AccountCommitment::commit(commitment_parameters, &commit_input, &private_key.r_pk)?,
        })
    }
}

impl<C: DPCComponents> ToBytes for AccountPublicKey<C> {
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

impl<C: DPCComponents> FromStr for AccountPublicKey<C> {
    type Err = AccountError;

    /// Reads in an account public key string.
    fn from_str(public_key: &str) -> Result<Self, Self::Err> {
        if public_key.len() != 63 {
            return Err(AccountError::InvalidCharacterLength(public_key.len()));
        }

        let prefix = &public_key.to_lowercase()[0..4];
        if prefix != account_format::PUBLIC_KEY_PREFIX.to_string() {
            return Err(AccountError::InvalidPrefix(prefix.to_string()));
        };

        let bech32 = Bech32::from_str(&public_key)?;
        if bech32.data().is_empty() {
            return Err(AccountError::InvalidByteLength(0));
        }

        let buffer = Vec::from_base32(&bech32.data())?;
        Ok(Self::read(&buffer[..])?)
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
        write!(f, "AccountPublicKey {{ commitment: {:?} }}", self.commitment)
    }
}
