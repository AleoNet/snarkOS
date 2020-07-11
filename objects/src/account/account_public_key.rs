use crate::{account_format, AccountPrivateKey, AccountViewKey};
use snarkos_algorithms::crh::bytes_to_bits;
use snarkos_errors::objects::AccountError;
use snarkos_models::{algorithms::EncryptionScheme, dpc::DPCComponents};
use snarkos_utilities::{FromBytes, ToBytes};

use bech32::{Bech32, FromBase32, ToBase32};
use std::{
    fmt,
    io::{Read, Result as IoResult, Write},
    str::FromStr,
};

// TODO (howardwu): Remove this and put it in snarkos-utilities.
pub fn bits_to_bytes(bits: &[bool]) -> Vec<u8> {
    // Pad the bits if it not a correct size
    let mut bits = bits.to_vec();
    if bits.len() % 8 != 0 {
        let current_length = bits.len();
        for _ in 0..(8 - current_length % 8) {
            bits.push(false);
        }
    }
    let mut bytes = Vec::with_capacity(bits.len() / 8);
    for bits in bits.chunks(8) {
        let mut result = 0u8;
        for (i, bit) in bits.iter().enumerate() {
            let bit_value = *bit as u8;
            result = result + (bit_value << i as u8);
        }
        bytes.push(result);
    }
    bytes
}

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
    /// Derives the account address from an account private key.
    pub fn from_private_key(
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

    /// Derives the account address from an account view key.
    pub fn from_view_key(
        encryption_parameters: &C::AccountEncryption,
        view_key: &AccountViewKey<C>,
    ) -> Result<Self, AccountError> {
        let encryption_key = <C::AccountEncryption as EncryptionScheme>::generate_public_key(
            encryption_parameters,
            &view_key.decryption_key,
        );

        Ok(Self { encryption_key })
    }

    pub fn into_repr(&self) -> &<C::AccountEncryption as EncryptionScheme>::PublicKey {
        &self.encryption_key
    }
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
        let mut encryption_key_bits = bytes_to_bits(&buffer);

        // Extract the bit above the MSB of the encryption key as the y_high bit.
        let size_in_bits = <C::AccountEncryption as EncryptionScheme>::public_key_size_in_bits();
        assert!(size_in_bits < encryption_key_bits.len());
        let y_high = encryption_key_bits[size_in_bits];

        // Zero the y_high bit position in the encryption key.
        encryption_key_bits[size_in_bits] = false;

        // Add the y_high indicator as an additional byte for the reader.
        let mut encryption_key = bits_to_bytes(&encryption_key_bits);
        encryption_key.push(y_high as u8);

        Ok(Self::read(&encryption_key[..])?)
    }
}

impl<C: DPCComponents> fmt::Display for AccountPublicKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write the encryption key to a buffer.
        let mut encryption_key = [0u8; 33];
        self.encryption_key
            .write(&mut encryption_key[0..33])
            .expect("encryption_key formatting failed");

        let mut encryption_key_bits = bytes_to_bits(&encryption_key[0..32]);
        let y_high = encryption_key[32] != 0;

        // Set the bit above the MSB of the encryption key as the y_high indicator bit.
        let size_in_bits = <C::AccountEncryption as EncryptionScheme>::public_key_size_in_bits();
        assert!(size_in_bits < encryption_key_bits.len());
        encryption_key_bits[size_in_bits] = y_high;

        let address = bits_to_bytes(&encryption_key_bits);

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
