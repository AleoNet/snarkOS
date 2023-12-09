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

use snarkvm::{
    circuit::prelude::PrimeField,
    console::{
        account::{Address, PrivateKey, Signature},
        prelude::{Environment, Uniform},
        types::Field,
    },
};

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use colored::Colorize;
use core::str::FromStr;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use rayon::prelude::*;

type Network = snarkvm::prelude::Testnet3;

/// Commands to manage Aleo accounts.
#[derive(Debug, Parser)]
pub enum Account {
    /// Generates a new Aleo account
    New {
        /// Seed the RNG with a numeric value
        #[clap(short = 's', long)]
        seed: Option<String>,
        /// Try until an address with the vanity string is found
        #[clap(short = 'v', long)]
        vanity: Option<String>,
    },
    Sign {
        /// Private key to use for the signature
        #[clap(short = 'k', long)]
        key: String,
        /// Message to sign
        #[clap(short = 'm', long)]
        message: String,
        /// Seed the RNG with a numeric value
        #[clap(short = 's', long)]
        seed: Option<String>,
    },
    Verify {
        /// Address to use for verification
        #[clap(short = 'a', long)]
        address: String,
        /// Signature to verify
        #[clap(short = 's', long)]
        signature: String,
        /// Message to verify the signature against
        #[clap(short = 'm', long)]
        message: String,
    },
}

impl Account {
    pub fn parse(self) -> Result<String> {
        match self {
            Self::New { seed, vanity } => {
                // Ensure only the seed or the vanity string is specified.
                if seed.is_some() && vanity.is_some() {
                    bail!("Cannot specify both the '--seed' and '--vanity' flags");
                }

                // Generate a vanity account.
                if let Some(vanity) = vanity {
                    Self::new_vanity(&vanity)
                }
                // Default to generating a normal account, with an optional seed.
                else {
                    Self::new_seeded(seed)
                }
            }
            Self::Sign { key, message, seed } => Self::sign(key, message, seed),
            Self::Verify { address, signature, message } => Self::verify(address, signature, message),
        }
    }

    /// Generates a new Aleo account with the given vanity string.
    fn new_vanity(vanity: &str) -> Result<String> {
        // A closure to generate a new Aleo account.
        let sample_account = || snarkos_account::Account::<Network>::new(&mut rand::thread_rng());

        const ITERATIONS: u128 = u16::MAX as u128;
        const ITERATIONS_STR: &str = "65,535";

        // Ensure the vanity string is valid.
        if !crate::helpers::is_in_bech32m_charset(vanity) {
            bail!(
                "The vanity string '{vanity}' contains invalid bech32m characters. Try using characters from the bech32m character set: {}",
                crate::helpers::BECH32M_CHARSET
            );
        }

        // Output a message if the character set is more than 4 characters.
        if vanity.len() > 4 {
            let message =
                format!(" The vanity string '{vanity}' contains 5 or more characters and will take a while to find.\n");
            println!("{}", message.yellow());
        }

        loop {
            // Initialize a timer.
            let timer = std::time::Instant::now();

            // Generates bech32m addresses in parallel until one is found that
            // includes the desired vanity string at the start or end of the address.
            let account = (0..ITERATIONS).into_par_iter().find_map_any(|_| {
                // Initialize the result.
                let mut account = None;
                // Sample a random account.
                if let Ok(candidate) = sample_account() {
                    // Encode the address as a bech32m string.
                    let address = candidate.address().to_string();
                    // Set the candidate if the address includes the desired vanity string
                    // at the start or end of the address.
                    if crate::helpers::has_vanity_string(&address, vanity) {
                        account = Some(candidate);
                    }
                }
                // Return the result.
                account
            });

            // Return the result if a candidate was found.
            if let Some(account) = account {
                println!(); // Add a newline for formatting.
                return Ok(account.to_string());
            } else {
                let rate = ITERATIONS / timer.elapsed().as_millis();
                let rate = format!("[{rate} a/ms]");
                println!(" {} Sampled {ITERATIONS_STR} accounts, searching...", rate.dimmed());
            }
        }
    }

    /// Generates a new Aleo account with an optional seed.
    fn new_seeded(seed: Option<String>) -> Result<String> {
        // Recover the seed.
        let seed = match seed {
            // Recover the field element deterministically.
            Some(seed) => {
                Field::new(<Network as Environment>::Field::from_str(&seed).map_err(|e| anyhow!("Invalid seed - {e}"))?)
            }
            // Sample a random field element.
            None => Field::rand(&mut ChaChaRng::from_entropy()),
        };
        // Recover the private key from the seed as a field element.
        let private_key =
            PrivateKey::try_from(seed).map_err(|_| anyhow!("Failed to convert the seed into a valid private key"))?;
        // Construct the account.
        let account = snarkos_account::Account::<Network>::try_from(private_key)?;
        // Print the new Aleo account.
        Ok(account.to_string())
    }

    // Sign a message with an Aleo private key
    fn sign(key: String, message: String, seed: Option<String>) -> Result<String> {
        // Recover the seed.
        let mut rng = match seed {
            // Recover the field element deterministically.
            Some(seed) => {
                let field: Field<_> = Field::<Network>::new(
                    <Network as Environment>::Field::from_str(&seed).map_err(|e| anyhow!("Invalid seed - {e}"))?,
                );
                let seed_bigint = field.to_bigint().0;

                let mut seed_bytes = [0u8; 32];
                seed_bytes[0..8].copy_from_slice(&seed_bigint[0].to_le_bytes());
                seed_bytes[8..16].copy_from_slice(&seed_bigint[1].to_le_bytes());
                seed_bytes[16..24].copy_from_slice(&seed_bigint[2].to_le_bytes());
                seed_bytes[24..32].copy_from_slice(&seed_bigint[3].to_le_bytes());
                ChaChaRng::from_seed(seed_bytes)
            }
            // Sample a random field element.
            None => ChaChaRng::from_entropy(),
        };

        // Parse the private key
        let private_key =
            PrivateKey::<Network>::from_str(&key).map_err(|_| anyhow!("Failed to parse a valid private key"))?;
        // Sign the message
        let signature = private_key
            .sign_bytes(message.as_bytes(), &mut rng)
            .map_err(|_| anyhow!("Failed to sign the message"))?
            .to_string();
        // Return the signature as a string
        Ok(signature)
    }

    // Verify a signature with an Aleo address
    fn verify(address: String, signature: String, message: String) -> Result<String> {
        // Parse the address
        let address = Address::<Network>::from_str(&address).map_err(|_| anyhow!("Failed to parse a valid address"))?;
        // Parse the signature
        let signature =
            Signature::<Network>::from_str(&signature).map_err(|_| anyhow!("Failed to parse a valid signature"))?;
        // Verify the signature
        let verified = signature.verify_bytes(&address, message.as_bytes());

        // Return the verification result
        Ok(if verified { "verified" } else { "invalid" }.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::Account;

    use colored::Colorize;

    #[test]
    fn test_new() {
        for _ in 0..3 {
            let account = Account::New { seed: None, vanity: None };
            assert!(account.parse().is_ok());
        }
    }

    #[test]
    fn test_new_seeded() {
        let seed = Some(1231275789u64.to_string());

        let mut expected = format!(
            " {:>12}  {}\n",
            "Private Key".cyan().bold(),
            "APrivateKey1zkp2n22c19hNdGF8wuEoQcuiyuWbquY6up4CtG5DYKqPX2X"
        );
        expected += &format!(
            " {:>12}  {}\n",
            "View Key".cyan().bold(),
            "AViewKey1pNxZHn79XVJ4D2WG5Vn2YWsAzf5wzAs3dAuQtUAmUFF7"
        );
        expected += &format!(
            " {:>12}  {}",
            "Address".cyan().bold(),
            "aleo1uxl69laseuv3876ksh8k0nd7tvpgjt6ccrgccedpjk9qwyfensxst9ftg5"
        );

        let vanity = None;
        let account = Account::New { seed, vanity };
        let actual = account.parse().unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_seeded_with_256bits_input() {
        let seed = Some("38868010450269069756484274649022187108349082664538872491798902858296683054657".to_string());

        let mut expected = format!(
            " {:>12}  {}\n",
            "Private Key".cyan().bold(),
            "APrivateKey1zkp61PAYmrYEKLtRWeWhUoDpFnGLNuHrCciSqN49T86dw3p"
        );
        expected += &format!(
            " {:>12}  {}\n",
            "View Key".cyan().bold(),
            "AViewKey1eYEGtb78FVg38SSYyzAeXnBdnWCba5t5YxUxtkTtvNAE"
        );
        expected += &format!(
            " {:>12}  {}",
            "Address".cyan().bold(),
            "aleo1zecnqchckrzw7dlsyf65g6z5le2rmys403ecwmcafrag0e030yxqrnlg8j"
        );

        let vanity = None;
        let account = Account::New { seed, vanity };
        let actual = account.parse().unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_signature() {
        let key = "APrivateKey1zkp61PAYmrYEKLtRWeWhUoDpFnGLNuHrCciSqN49T86dw3p".to_string();
        let message = "Hello, world!".to_string();
        let account = Account::Sign { key, message, seed: None };
        assert!(account.parse().is_ok());
    }

    #[test]
    fn test_seeded_signature() {
        let seed = Some("38868010450269069756484274649022187108349082664538872491798902858296683054657".to_string());
        let key = "APrivateKey1zkp61PAYmrYEKLtRWeWhUoDpFnGLNuHrCciSqN49T86dw3p".to_string();
        let message = "Hello, world!".to_string();
        let expected = "sign1t2hsaqfhcgvsfg2q3q2stxsffyrvdx98pl0ddkdqngqqtn3vsuprhkv9tkeyzs878ccqp62mfptvvp7m5hjcfnf06cc9pu4khxtkkp8esm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qkksrwh0";
        let account = Account::Sign { key, message, seed };
        let actual = account.parse().unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_verify() {
        // test signature of "Hello, world!"
        let address = "aleo1zecnqchckrzw7dlsyf65g6z5le2rmys403ecwmcafrag0e030yxqrnlg8j";
        let signature = "sign1nnvrjlksrkxdpwsrw8kztjukzhmuhe5zf3srk38h7g32u4kqtqpxn3j5a6k8zrqcfx580a96956nsjvluzt64cqf54pdka9mgksfqp8esm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qkwsnaqq".to_string();
        let message = "Hello, world!".to_string();
        let account = Account::Verify { address: address.to_string(), signature, message };
        let actual = account.parse().unwrap();
        assert_eq!("verified", actual);

        // test signature of "Hello, world!" against the message "Different Message"
        let signature = "sign1nnvrjlksrkxdpwsrw8kztjukzhmuhe5zf3srk38h7g32u4kqtqpxn3j5a6k8zrqcfx580a96956nsjvluzt64cqf54pdka9mgksfqp8esm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qkwsnaqq".to_string();
        let message = "Different Message".to_string();
        let account = Account::Verify { address: address.to_string(), signature, message };
        let actual = account.parse().unwrap();
        assert_eq!("invalid", actual);

        // test signature of "Hello, world!" against the wrong address
        let signature = "sign1nnvrjlksrkxdpwsrw8kztjukzhmuhe5zf3srk38h7g32u4kqtqpxn3j5a6k8zrqcfx580a96956nsjvluzt64cqf54pdka9mgksfqp8esm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qkwsnaqq".to_string();
        let message = "Hello, world!".to_string();
        let wrong_address = "aleo1uxl69laseuv3876ksh8k0nd7tvpgjt6ccrgccedpjk9qwyfensxst9ftg5".to_string();
        let account = Account::Verify { address: wrong_address, signature, message };
        let actual = account.parse().unwrap();
        assert_eq!("invalid", actual);

        // test a valid signature of "Different Message"
        let signature = "sign1424ztyt9hcm77nq450gvdszrvtg9kvhc4qadg4nzy9y0ah7wdqq7t36cxal42p9jj8e8pjpmc06lfev9nvffcpqv0cxwyr0a2j2tjqlesm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qk3yrr50".to_string();
        let message = "Different Message".to_string();
        let account = Account::Verify { address: address.to_string(), signature, message };
        let actual = account.parse().unwrap();
        assert_eq!("verified", actual);
    }
}
