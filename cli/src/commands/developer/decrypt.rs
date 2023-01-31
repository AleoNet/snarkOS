// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use super::Network;

use snarkvm::{
    console::program::Ciphertext,
    prelude::{Record, ViewKey},
};

use anyhow::{bail, Result};
use clap::Parser;
use std::str::FromStr;

/// Decrypts a record ciphertext.
#[derive(Debug, Parser)]
pub struct Decrypt {
    /// The record ciphertext to decrypt.
    #[clap(short = 'c', long, help = "The record ciphertext to decrypt.")]
    pub ciphertext: String,
    /// The view key used to decrypt the record ciphertext.
    #[clap(short = 'v', long, help = "The view key used to decrypt the ciphertext.")]
    pub view_key: String,
}

impl Decrypt {
    pub fn parse(self) -> Result<String> {
        // Decrypt the ciphertext.
        Self::decrypt_ciphertext(&self.ciphertext, &self.view_key)
    }

    /// Decrypts the ciphertext record with provided the view key.
    fn decrypt_ciphertext(ciphertext: &str, view_key: &str) -> Result<String> {
        // Parse the ciphertext record.
        let ciphertext_record = Record::<Network, Ciphertext<Network>>::from_str(ciphertext)?;

        // Parse the account view key.
        let view_key = ViewKey::<Network>::from_str(view_key)?;

        match ciphertext_record.decrypt(&view_key) {
            Ok(plaintext_record) => Ok(plaintext_record.to_string()),
            Err(_) => bail!("Invalid view key for the provided record ciphertext"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt() {}

    #[test]
    fn test_failed_decryption() {}
}
