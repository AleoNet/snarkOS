// Copyright 2024 Aleo Network Foundation
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

use snarkvm::console::{
    account::{Address, PrivateKey, Signature},
    network::{CanaryV0, MainnetV0, Network, TestnetV0},
    prelude::{Environment, Uniform},
    program::{ToFields, Value},
    types::Field,
};

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use colored::Colorize;
use core::str::FromStr;
use crossterm::ExecutableCommand;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use rayon::prelude::*;
use std::{
    io::{Read, Write},
    path::PathBuf,
};
use zeroize::Zeroize;

/// Commands to manage Aleo accounts.
#[derive(Debug, Parser, Zeroize)]
pub enum Account {
    /// Generates a new Aleo account
    New {
        /// Specify the network of the account
        #[clap(default_value = "0", long = "network")]
        network: u16,
        /// Seed the RNG with a numeric value
        #[clap(short = 's', long)]
        seed: Option<String>,
        /// Try until an address with the vanity string is found
        #[clap(short = 'v', long)]
        vanity: Option<String>,
        /// Print sensitive information (such as the private key) discreetly in an alternate screen
        #[clap(long)]
        discreet: bool,
    },
    Sign {
        /// Specify the network of the private key to sign with
        #[clap(default_value = "0", long = "network")]
        network: u16,
        /// Specify the account private key of the node
        #[clap(long = "private-key")]
        private_key: Option<String>,
        /// Specify the path to a file containing the account private key of the node
        #[clap(long = "private-key-file")]
        private_key_file: Option<String>,
        /// Message (Aleo value) to sign
        #[clap(short = 'm', long)]
        message: String,
        /// When enabled, parses the message as bytes instead of Aleo literals
        #[clap(short = 'r', long)]
        raw: bool,
    },
    Verify {
        /// Specify the network of the signature to verify
        #[clap(default_value = "0", long = "network")]
        network: u16,
        /// Address to use for verification
        #[clap(short = 'a', long)]
        address: String,
        /// Signature to verify
        #[clap(short = 's', long)]
        signature: String,
        /// Message (Aleo value) to verify the signature against
        #[clap(short = 'm', long)]
        message: String,
        /// When enabled, parses the message as bytes instead of Aleo literals
        #[clap(short = 'r', long)]
        raw: bool,
    },
}

/// Parse a raw Aleo input into fields
fn aleo_literal_to_fields<N: Network>(input: &str) -> Result<Vec<Field<N>>> {
    Value::<N>::from_str(input)?.to_fields()
}

impl Account {
    pub fn parse(self) -> Result<String> {
        match self {
            Self::New { network, seed, vanity, discreet } => {
                // Ensure only the seed or the vanity string is specified.
                if seed.is_some() && vanity.is_some() {
                    bail!("Cannot specify both the '--seed' and '--vanity' flags");
                }

                match vanity {
                    // Generate a vanity account for the specified network.
                    Some(vanity) => match network {
                        MainnetV0::ID => Self::new_vanity::<MainnetV0>(vanity.as_str(), discreet),
                        TestnetV0::ID => Self::new_vanity::<TestnetV0>(vanity.as_str(), discreet),
                        CanaryV0::ID => Self::new_vanity::<CanaryV0>(vanity.as_str(), discreet),
                        unknown_id => bail!("Unknown network ID ({unknown_id})"),
                    },
                    // Generate a seeded account for the specified network.
                    None => match network {
                        MainnetV0::ID => Self::new_seeded::<MainnetV0>(seed, discreet),
                        TestnetV0::ID => Self::new_seeded::<TestnetV0>(seed, discreet),
                        CanaryV0::ID => Self::new_seeded::<CanaryV0>(seed, discreet),
                        unknown_id => bail!("Unknown network ID ({unknown_id})"),
                    },
                }
            }
            Self::Sign { network, message, raw, private_key, private_key_file } => {
                let key = match (private_key, private_key_file) {
                    (Some(private_key), None) => private_key,
                    (None, Some(private_key_file)) => {
                        let path = private_key_file.parse::<PathBuf>().map_err(|e| anyhow!("Invalid path - {e}"))?;
                        std::fs::read_to_string(path)?.trim().to_string()
                    }
                    (None, None) => bail!("Missing the '--private-key' or '--private-key-file' argument"),
                    (Some(_), Some(_)) => {
                        bail!("Cannot specify both the '--private-key' and '--private-key-file' flags")
                    }
                };

                // Sign the message for the specified network.
                match network {
                    MainnetV0::ID => Self::sign::<MainnetV0>(key, message, raw),
                    TestnetV0::ID => Self::sign::<TestnetV0>(key, message, raw),
                    CanaryV0::ID => Self::sign::<CanaryV0>(key, message, raw),
                    unknown_id => bail!("Unknown network ID ({unknown_id})"),
                }
            }
            Self::Verify { network, address, signature, message, raw } => {
                // Verify the signature for the specified network.
                match network {
                    MainnetV0::ID => Self::verify::<MainnetV0>(address, signature, message, raw),
                    TestnetV0::ID => Self::verify::<TestnetV0>(address, signature, message, raw),
                    CanaryV0::ID => Self::verify::<CanaryV0>(address, signature, message, raw),
                    unknown_id => bail!("Unknown network ID ({unknown_id})"),
                }
            }
        }
    }

    /// Generates a new Aleo account with the given vanity string.
    fn new_vanity<N: Network>(vanity: &str, discreet: bool) -> Result<String> {
        // A closure to generate a new Aleo account.
        let sample_account = || snarkos_account::Account::<N>::new(&mut rand::thread_rng());

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
                if !discreet {
                    return Ok(account.to_string());
                }
                display_string_discreetly(
                    &format!("{:>12}  {}", "Private Key".cyan().bold(), account.private_key()),
                    "### Do not share or lose this private key! Press any key to complete. ###",
                )
                .unwrap();
                let account_info = format!(
                    " {:>12}  {}\n {:>12}  {}",
                    "View Key".cyan().bold(),
                    account.view_key(),
                    "Address".cyan().bold(),
                    account.address()
                );
                return Ok(account_info);
            } else {
                let rate = ITERATIONS / timer.elapsed().as_millis();
                let rate = format!("[{rate} a/ms]");
                println!(" {} Sampled {ITERATIONS_STR} accounts, searching...", rate.dimmed());
            }
        }
    }

    /// Generates a new Aleo account with an optional seed.
    fn new_seeded<N: Network>(seed: Option<String>, discreet: bool) -> Result<String> {
        // Recover the seed.
        let seed = match seed {
            // Recover the field element deterministically.
            Some(seed) => {
                Field::new(<N as Environment>::Field::from_str(&seed).map_err(|e| anyhow!("Invalid seed - {e}"))?)
            }
            // Sample a random field element.
            None => Field::rand(&mut ChaChaRng::from_entropy()),
        };
        // Recover the private key from the seed as a field element.
        let private_key =
            PrivateKey::try_from(seed).map_err(|_| anyhow!("Failed to convert the seed into a valid private key"))?;
        // Construct the account.
        let account = snarkos_account::Account::<N>::try_from(private_key)?;
        // Print the new Aleo account.
        if !discreet {
            return Ok(account.to_string());
        }
        display_string_discreetly(
            &format!("{:>12}  {}", "Private Key".cyan().bold(), account.private_key()),
            "### Do not share or lose this private key! Press any key to complete. ###",
        )
        .unwrap();
        let account_info = format!(
            " {:>12}  {}\n {:>12}  {}",
            "View Key".cyan().bold(),
            account.view_key(),
            "Address".cyan().bold(),
            account.address()
        );
        Ok(account_info)
    }

    // Sign a message with an Aleo private key
    fn sign<N: Network>(key: String, message: String, raw: bool) -> Result<String> {
        // Sample a random field element.
        let mut rng = ChaChaRng::from_entropy();

        // Parse the private key
        let private_key =
            PrivateKey::<N>::from_str(&key).map_err(|_| anyhow!("Failed to parse a valid private key"))?;
        // Sign the message
        let signature = if raw {
            private_key.sign_bytes(message.as_bytes(), &mut rng)
        } else {
            let fields =
                aleo_literal_to_fields::<N>(&message).map_err(|_| anyhow!("Failed to parse a valid Aleo literal"))?;
            private_key.sign(&fields, &mut rng)
        }
        .map_err(|_| anyhow!("Failed to sign the message"))?
        .to_string();
        // Return the signature as a string
        Ok(signature)
    }

    // Verify a signature with an Aleo address
    fn verify<N: Network>(address: String, signature: String, message: String, raw: bool) -> Result<String> {
        // Parse the address
        let address = Address::<N>::from_str(&address).map_err(|_| anyhow!("Failed to parse a valid address"))?;
        // Parse the signature
        let signature =
            Signature::<N>::from_str(&signature).map_err(|_| anyhow!("Failed to parse a valid signature"))?;
        // Verify the signature
        let verified = if raw {
            signature.verify_bytes(&address, message.as_bytes())
        } else {
            let fields =
                aleo_literal_to_fields(&message).map_err(|_| anyhow!("Failed to parse a valid Aleo literal"))?;
            signature.verify(&address, &fields)
        };

        // Return the verification result
        match verified {
            true => Ok("✅ The signature is valid".to_string()),
            false => bail!("❌ The signature is invalid"),
        }
    }
}

// Print the string to an alternate screen, so that the string won't been printed to the terminal.
fn display_string_discreetly(discreet_string: &str, continue_message: &str) -> Result<()> {
    use crossterm::{
        style::Print,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    };
    let mut stdout = std::io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    // print msg on the alternate screen
    stdout.execute(Print(format!("{discreet_string}\n{continue_message}")))?;
    stdout.flush()?;
    wait_for_keypress();
    stdout.execute(LeaveAlternateScreen)?;
    Ok(())
}

fn wait_for_keypress() {
    let mut single_key = [0u8];
    std::io::stdin().read_exact(&mut single_key).unwrap();
}

#[cfg(test)]
mod tests {
    use crate::commands::Account;

    use colored::Colorize;

    #[test]
    fn test_new() {
        for _ in 0..3 {
            let account = Account::New { network: 0, seed: None, vanity: None, discreet: false };
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
        let account = Account::New { network: 0, seed, vanity, discreet: false };
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
        let account = Account::New { network: 0, seed, vanity, discreet: false };
        let actual = account.parse().unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_signature_raw() {
        let key = "APrivateKey1zkp61PAYmrYEKLtRWeWhUoDpFnGLNuHrCciSqN49T86dw3p".to_string();
        let message = "Hello, world!".to_string();
        let account = Account::Sign { network: 0, private_key: Some(key), private_key_file: None, message, raw: true };
        assert!(account.parse().is_ok());
    }

    #[test]
    fn test_signature() {
        let key = "APrivateKey1zkp61PAYmrYEKLtRWeWhUoDpFnGLNuHrCciSqN49T86dw3p".to_string();
        let message = "5field".to_string();
        let account = Account::Sign { network: 0, private_key: Some(key), private_key_file: None, message, raw: false };
        assert!(account.parse().is_ok());
    }

    #[test]
    fn test_signature_fail() {
        let key = "APrivateKey1zkp61PAYmrYEKLtRWeWhUoDpFnGLNuHrCciSqN49T86dw3p".to_string();
        let message = "not a literal value".to_string();
        let account = Account::Sign { network: 0, private_key: Some(key), private_key_file: None, message, raw: false };
        assert!(account.parse().is_err());
    }

    #[test]
    fn test_verify_raw() {
        // test signature of "Hello, world!"
        let address = "aleo1zecnqchckrzw7dlsyf65g6z5le2rmys403ecwmcafrag0e030yxqrnlg8j";
        let signature = "sign1nnvrjlksrkxdpwsrw8kztjukzhmuhe5zf3srk38h7g32u4kqtqpxn3j5a6k8zrqcfx580a96956nsjvluzt64cqf54pdka9mgksfqp8esm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qkwsnaqq".to_string();
        let message = "Hello, world!".to_string();
        let account = Account::Verify { network: 0, address: address.to_string(), signature, message, raw: true };
        let actual = account.parse();
        assert!(actual.is_ok());

        // test signature of "Hello, world!" against the message "Different Message"
        let signature = "sign1nnvrjlksrkxdpwsrw8kztjukzhmuhe5zf3srk38h7g32u4kqtqpxn3j5a6k8zrqcfx580a96956nsjvluzt64cqf54pdka9mgksfqp8esm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qkwsnaqq".to_string();
        let message = "Different Message".to_string();
        let account = Account::Verify { network: 0, address: address.to_string(), signature, message, raw: true };
        let actual = account.parse();
        assert!(actual.is_err());

        // test signature of "Hello, world!" against the wrong address
        let signature = "sign1nnvrjlksrkxdpwsrw8kztjukzhmuhe5zf3srk38h7g32u4kqtqpxn3j5a6k8zrqcfx580a96956nsjvluzt64cqf54pdka9mgksfqp8esm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qkwsnaqq".to_string();
        let message = "Hello, world!".to_string();
        let wrong_address = "aleo1uxl69laseuv3876ksh8k0nd7tvpgjt6ccrgccedpjk9qwyfensxst9ftg5".to_string();
        let account = Account::Verify { network: 0, address: wrong_address, signature, message, raw: true };
        let actual = account.parse();
        assert!(actual.is_err());

        // test a valid signature of "Different Message"
        let signature = "sign1424ztyt9hcm77nq450gvdszrvtg9kvhc4qadg4nzy9y0ah7wdqq7t36cxal42p9jj8e8pjpmc06lfev9nvffcpqv0cxwyr0a2j2tjqlesm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qk3yrr50".to_string();
        let message = "Different Message".to_string();
        let account = Account::Verify { network: 0, address: address.to_string(), signature, message, raw: true };
        let actual = account.parse();
        assert!(actual.is_ok());
    }

    #[test]
    fn test_verify() {
        // test signature of 5u8
        let address = "aleo1zecnqchckrzw7dlsyf65g6z5le2rmys403ecwmcafrag0e030yxqrnlg8j";
        let signature = "sign1j7swjfnyujt2vme3ulu88wdyh2ddj85arh64qh6c6khvrx8wvsp8z9wtzde0sahqj2qwz8rgzt803c0ceega53l4hks2mf5sfsv36qhesm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qkdetews".to_string();
        let message = "5field".to_string();
        let account = Account::Verify { network: 0, address: address.to_string(), signature, message, raw: false };
        let actual = account.parse();
        assert!(actual.is_ok());

        // test signature of 5u8 against the message 10u8
        let signature = "sign1j7swjfnyujt2vme3ulu88wdyh2ddj85arh64qh6c6khvrx8wvsp8z9wtzde0sahqj2qwz8rgzt803c0ceega53l4hks2mf5sfsv36qhesm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qkdetews".to_string();
        let message = "10field".to_string();
        let account = Account::Verify { network: 0, address: address.to_string(), signature, message, raw: false };
        let actual = account.parse();
        assert!(actual.is_err());

        // test signature of 5u8 against the wrong address
        let signature = "sign1j7swjfnyujt2vme3ulu88wdyh2ddj85arh64qh6c6khvrx8wvsp8z9wtzde0sahqj2qwz8rgzt803c0ceega53l4hks2mf5sfsv36qhesm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qkdetews".to_string();
        let message = "5field".to_string();
        let wrong_address = "aleo1uxl69laseuv3876ksh8k0nd7tvpgjt6ccrgccedpjk9qwyfensxst9ftg5".to_string();
        let account = Account::Verify { network: 0, address: wrong_address, signature, message, raw: false };
        let actual = account.parse();
        assert!(actual.is_err());

        // test a valid signature of 10u8
        let signature = "sign1t9v2t5tljk8pr5t6vkcqgkus0a3v69vryxmfrtwrwg0xtj7yv5qj2nz59e5zcyl50w23lhntxvt6vzeqfyu6dt56698zvfj2l6lz6q0esm5elrqqunzqzmac7kzutl6zk7mqht3c0m9kg4hklv7h2js0qmxavwnpuwyl4lzldl6prs4qeqy9wxyp8y44nnydg3h8sg6ue99qk8rh9kt".to_string();
        let message = "10field".to_string();
        let account = Account::Verify { network: 0, address: address.to_string(), signature, message, raw: false };
        let actual = account.parse();
        assert!(actual.is_ok());
    }
}
