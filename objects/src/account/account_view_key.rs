use crate::AccountPrivateKey;
use snarkos_errors::objects::AccountError;
use snarkos_models::{algorithms::EncryptionScheme, dpc::DPCComponents};
use snarkos_utilities::{FromBytes, ToBytes};

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
pub struct AccountViewKey<C: DPCComponents> {
    pub decryption_key: <C::AccountEncryption as EncryptionScheme>::PrivateKey,
}

impl<C: DPCComponents> AccountViewKey<C> {
    /// Creates a new account view key from an account private key.
    pub fn from_private_key(
        signature_parameters: &C::AccountSignature,
        commitment_parameters: &C::AccountCommitment,
        private_key: &AccountPrivateKey<C>,
    ) -> Result<Self, AccountError> {
        let decryption_key = private_key.to_decryption_key(signature_parameters, commitment_parameters)?;

        Ok(Self { decryption_key })
    }
}

impl<C: DPCComponents> ToBytes for AccountViewKey<C> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.decryption_key.write(&mut writer)
    }
}

impl<C: DPCComponents> FromBytes for AccountViewKey<C> {
    /// Reads in an account view key buffer.
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let decryption_key = <C::AccountEncryption as EncryptionScheme>::PrivateKey::read(&mut reader)?;

        Ok(Self { decryption_key })
    }
}

impl<C: DPCComponents> fmt::Debug for AccountViewKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AccountViewKey {{ decryption_key: {:?} }}", self.decryption_key)
    }
}
