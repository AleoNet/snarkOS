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

use snarkvm::console::{
    account::PrivateKey,
    prelude::{Environment, Uniform},
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
use std::io::{Read, Write};

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
        /// Print sensitive information (such as the private key) discreetly in an alternate screen
        #[clap(long)]
        discreet: bool,
    },
}

impl Account {
    pub fn parse(self) -> Result<String> {
        match self {
            Self::New { seed, vanity, discreet } => {
                // Ensure only the seed or the vanity string is specified.
                if seed.is_some() && vanity.is_some() {
                    bail!("Cannot specify both the '--seed' and '--vanity' flags");
                }

                // Generate a vanity account.
                if let Some(vanity) = vanity {
                    Self::new_vanity(&vanity, discreet)
                }
                // Default to generating a normal account, with an optional seed.
                else {
                    Self::new_seeded(seed, discreet)
                }
            }
        }
    }

    /// Generates a new Aleo account with the given vanity string.
    fn new_vanity(vanity: &str, discreet: bool) -> Result<String> {
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
    fn new_seeded(seed: Option<String>, discreet: bool) -> Result<String> {
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
            let account = Account::New { seed: None, vanity: None, discreet: false };
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
        let account = Account::New { seed, vanity, discreet: false };
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
        let account = Account::New { seed, vanity, discreet: false };
        let actual = account.parse().unwrap();
        assert_eq!(expected, actual);
    }
}
