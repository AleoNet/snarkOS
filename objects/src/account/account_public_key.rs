use crate::{account_format, AccountPrivateKey};
use snarkos_errors::objects::AccountError;
use snarkos_models::{
    algorithms::{CommitmentScheme, EncryptionScheme},
    dpc::DPCComponents,
};
use snarkos_utilities::{to_bytes, FromBytes, ToBytes};

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
    pub encryption_key: <C::AccountEncryption as EncryptionScheme>::PublicKey,
}

impl<C: DPCComponents> AccountPublicKey<C> {
    /// Creates a new account address from an account private key.
    pub fn from(
        signature_parameters: &C::AccountSignature,
        commitment_parameters: &C::AccountCommitment,
        encryption_parameters: &C::AccountEncryption,
        private_key: &AccountPrivateKey<C>,
    ) -> Result<Self, AccountError> {
        let decryption_key = private_key.to_decryption_key(signature_parameters, commitment_parameters)?;
        let encryption_key =
            <C::AccountEncryption as EncryptionScheme>::generate_public_key(encryption_parameters, &decryption_key);

        Ok(Self { encryption_key })
    }

    pub fn into_repr(&self) -> &<C::AccountEncryption as EncryptionScheme>::PublicKey {
        &self.encryption_key
    }

    // pub fn as_commitment(&self) -> Result<<C::AccountCommitment as CommitmentScheme>::Output, AccountError> {
    //     let commitment_bytes = to_bytes![self.decryption_key]?;
    //     Ok(<C::AccountCommitment as CommitmentScheme>::Output::read(
    //         &commitment_bytes[..],
    //     )?)
    // }
}

impl<C: DPCComponents> ToBytes for AccountPublicKey<C> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.encryption_key.write(&mut writer)
    }
}

impl<C: DPCComponents> FromBytes for AccountPublicKey<C> {
    /// Reads in an account address buffer.
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let encryption_key: <C::AccountEncryption as EncryptionScheme>::PublicKey = FromBytes::read(&mut reader)?;

        Ok(Self { encryption_key })
    }
}

impl<C: DPCComponents> FromStr for AccountPublicKey<C> {
    type Err = AccountError;

    /// Reads in an account address string.
    fn from_str(public_key: &str) -> Result<Self, Self::Err> {
        if public_key.len() != 63 {
            return Err(AccountError::InvalidCharacterLength(public_key.len()));
        }

        let prefix = &public_key.to_lowercase()[0..4];
        if prefix != account_format::ADDRESS_PREFIX.to_string() {
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
        let mut address = [0u8; 32];
        self.encryption_key
            .write(&mut address[0..32])
            .expect("address formatting failed");

        let prefix = account_format::ADDRESS_PREFIX.to_string();

        let result = Bech32::new(prefix, address.to_base32());
        result.unwrap().fmt(f)
    }
}

impl<C: DPCComponents> fmt::Debug for AccountPublicKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AccountAddress {{ encryption_key: {:?} }}", self.encryption_key)
    }
}
