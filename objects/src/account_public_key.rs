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
    Debug(bound = "C: DPCComponents")
)]
pub struct AccountPublicKey<C: DPCComponents> {
    pub public_key: <C::AddressCommitment as CommitmentScheme>::Output,
    pub is_testnet: bool,
}

impl<C: DPCComponents> AccountPublicKey<C> {
    /// Creates a new account public key from an account private key. Defaults to a testnet account
    /// if no network indicator is provided.
    pub fn from(parameters: &C::AddressCommitment, private_key: &AccountPrivateKey<C>) -> Result<Self, AccountError> {
        // Construct the commitment input for the account public key.
        let commit_input = to_bytes![private_key.pk_sig, private_key.sk_prf, private_key.metadata]?;

        Ok(Self {
            public_key: C::AddressCommitment::commit(parameters, &commit_input, &private_key.r_pk)?,
            is_testnet: private_key.is_testnet,
        })
    }
}

impl<C: DPCComponents> ToBytes for AccountPublicKey<C> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.public_key.write(&mut writer)?;
        self.is_testnet.write(&mut writer)
    }
}

impl<C: DPCComponents> FromBytes for AccountPublicKey<C> {
    /// Reads in an account public key buffer. Defaults to a testnet account
    /// if no network indicator is provided.
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let public_key: <C::AddressCommitment as CommitmentScheme>::Output = FromBytes::read(&mut reader)?;
        let is_testnet: bool = match FromBytes::read(&mut reader) {
            Ok(is_testnet) => is_testnet,
            _ => true, // Defaults to testnet
        };

        Ok(Self { public_key, is_testnet })
    }
}

impl<C: DPCComponents> fmt::Display for AccountPublicKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut public_key = [0u8; 32];
        self.public_key
            .write(&mut public_key[0..32])
            .expect("public key formatting failed");

        let prefix = match self.is_testnet {
            true => account_format::PUBLIC_KEY_TESTNET.to_string(),
            false => account_format::PUBLIC_KEY_MAINNET.to_string(),
        };

        let result = Bech32::new(prefix, public_key.to_base32());
        result.unwrap().fmt(f)
    }
}
