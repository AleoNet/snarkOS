// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::CurrentNetwork;
use snarkvm::{
    console::program::Ciphertext,
    prelude::{Record, ViewKey},
};

use anyhow::{bail, Result};
use clap::Parser;
use std::str::FromStr;
use zeroize::Zeroize;

/// Decrypts a record ciphertext.
#[derive(Debug, Parser, Zeroize)]
pub struct Decrypt {
    /// The record ciphertext to decrypt.
    #[clap(short, long)]
    pub ciphertext: String,
    /// The view key used to decrypt the record ciphertext.
    #[clap(short, long)]
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
        let ciphertext_record = Record::<CurrentNetwork, Ciphertext<CurrentNetwork>>::from_str(ciphertext)?;

        // Parse the account view key.
        let view_key = ViewKey::<CurrentNetwork>::from_str(view_key)?;

        match ciphertext_record.decrypt(&view_key) {
            Ok(plaintext_record) => Ok(plaintext_record.to_string()),
            Err(_) => bail!("Invalid view key for the provided record ciphertext"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indexmap::IndexMap;
    use snarkvm::prelude::{
        Address,
        Entry,
        Field,
        Identifier,
        Literal,
        Network,
        Owner,
        Plaintext,
        PrivateKey,
        Scalar,
        TestRng,
        Uniform,
        ViewKey,
    };

    const ITERATIONS: usize = 1000;

    fn construct_ciphertext<N: Network>(
        view_key: ViewKey<N>,
        owner: Owner<N, Plaintext<N>>,
        rng: &mut TestRng,
    ) -> Result<Record<N, Ciphertext<N>>> {
        // Prepare the record.
        let randomizer = Scalar::rand(rng);
        let record = Record::<N, Plaintext<N>>::from_plaintext(
            owner,
            IndexMap::from_iter(
                vec![
                    (Identifier::from_str("a")?, Entry::Private(Plaintext::from(Literal::Field(Field::rand(rng))))),
                    (Identifier::from_str("b")?, Entry::Private(Plaintext::from(Literal::Scalar(Scalar::rand(rng))))),
                ]
                .into_iter(),
            ),
            N::g_scalar_multiply(&randomizer),
        )?;
        // Encrypt the record.
        let ciphertext = record.encrypt(randomizer)?;
        // Decrypt the record.
        assert_eq!(record, ciphertext.decrypt(&view_key)?);

        Ok(ciphertext)
    }

    #[test]
    fn test_decryption() {
        let mut rng = TestRng::default();

        for _ in 0..ITERATIONS {
            let private_key = PrivateKey::<CurrentNetwork>::new(&mut rng).unwrap();
            let view_key = ViewKey::try_from(private_key).unwrap();
            let address = Address::try_from(private_key).unwrap();

            // Construct the ciphertext.
            let owner = Owner::Private(Plaintext::from(Literal::Address(address)));
            let ciphertext = construct_ciphertext(view_key, owner, &mut rng).unwrap();

            // Decrypt the ciphertext.
            let expected_plaintext = ciphertext.decrypt(&view_key).unwrap();

            let decrypt = Decrypt { ciphertext: ciphertext.to_string(), view_key: view_key.to_string() };
            let plaintext = decrypt.parse().unwrap();

            // Check that the decryption is correct.
            assert_eq!(plaintext, expected_plaintext.to_string());
        }
    }

    // TODO (raychu86): Fix this test. https://github.com/AleoHQ/snarkVM/issues/1472
    #[ignore]
    #[test]
    fn test_failed_decryption() {
        let mut rng = TestRng::default();

        // Generate a view key that is unaffiliated with the ciphertext.
        let incorrect_private_key = PrivateKey::<CurrentNetwork>::new(&mut rng).unwrap();
        let incorrect_view_key = ViewKey::try_from(incorrect_private_key).unwrap();

        for _ in 0..ITERATIONS {
            let private_key = PrivateKey::<CurrentNetwork>::new(&mut rng).unwrap();
            let view_key = ViewKey::try_from(private_key).unwrap();
            let address = Address::try_from(private_key).unwrap();

            // Construct the ciphertext.
            let owner = Owner::Private(Plaintext::from(Literal::Address(address)));
            let ciphertext = construct_ciphertext::<CurrentNetwork>(view_key, owner, &mut rng).unwrap();

            // Enforce that the decryption fails.
            let decrypt = Decrypt { ciphertext: ciphertext.to_string(), view_key: incorrect_view_key.to_string() };
            assert!(decrypt.parse().is_err());
        }
    }
}
