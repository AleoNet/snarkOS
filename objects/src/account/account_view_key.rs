// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{account_format, AccountPrivateKey};
use snarkos_errors::objects::AccountError;
use snarkvm_models::{algorithms::EncryptionScheme, dpc::DPCComponents};
use snarkvm_utilities::{FromBytes, ToBytes};

use base58::{FromBase58, ToBase58};
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

impl<C: DPCComponents> FromStr for AccountViewKey<C> {
    type Err = AccountError;

    /// Reads in an account view key string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let data = s.from_base58()?;
        if data.len() != 39 {
            return Err(AccountError::InvalidByteLength(data.len()));
        }

        if data[0..7] != account_format::VIEW_KEY_PREFIX {
            return Err(AccountError::InvalidPrefixBytes(data[0..7].to_vec()));
        }

        let mut reader = &data[7..];
        let decryption_key: <C::AccountEncryption as EncryptionScheme>::PrivateKey = FromBytes::read(&mut reader)?;

        Ok(Self { decryption_key })
    }
}

impl<C: DPCComponents> fmt::Display for AccountViewKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut view_key = [0u8; 39];
        let prefix = account_format::VIEW_KEY_PREFIX;

        view_key[0..7].copy_from_slice(&prefix);

        self.decryption_key
            .write(&mut view_key[7..39])
            .expect("decryption_key formatting failed");

        write!(f, "{}", view_key.to_base58())
    }
}
