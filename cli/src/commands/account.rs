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

use snarkvm::console::{
    account::PrivateKey,
    prelude::{Environment, Uniform},
    types::Field,
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
                let rate = format!("[{} a/ms]", rate);
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
}
