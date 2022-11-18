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

use anyhow::Result;
use clap::Parser;
use num_bigint::BigInt;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use snarkvm::prelude::PrivateKey;

type Network = snarkvm::prelude::Testnet3;

/// Commands to manage Aleo accounts.
#[derive(Debug, Parser)]
pub enum Account {
    /// Generates a new Aleo account
    New {
        /// Seed the RNG with a numeric value
        #[clap(short = 's', long)]
        seed: Option<BigInt>,
    },
}

impl Account {
    pub fn parse(self) -> Result<String> {
        match self {
            Self::New { seed } => {
                // Sample a new Aleo account.
                let account = snarkos_account::Account::from(match seed {
                    Some(bigint) => {
                        let mut bytes = bigint.to_bytes_be().1;
                        bytes.truncate(32);
                        bytes.resize(32, 0);
                        let seed: [u8; 32] =
                            bytes.try_into().expect("Invalid seed: Failed to convert the seed into a 32 bytes number.");
                        PrivateKey::<Network>::new(&mut ChaChaRng::from_seed(seed))?
                    }
                    None => PrivateKey::new(&mut rand::thread_rng())?,
                })?;
                // Print the new Aleo account.
                Ok(account.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::Account;
    use colored::Colorize;
    use num_bigint::{BigInt, ToBigInt};
    use std::str::FromStr;

    #[test]
    fn test_new() {
        for _ in 0..3 {
            let account = Account::New { seed: None };
            assert!(account.parse().is_ok());
        }
    }

    #[test]
    fn test_new_seeded() {
        let seed = 1231275789u64.to_bigint();
        let mut expected = format!(
            " {:>12}  {}\n",
            "Private Key".cyan().bold(),
            "APrivateKey1zkp9t1en6qJauvSGGngoEvFSeER2T7A9Yx6NbGxMqEoxMjT"
        );
        expected += &format!(
            " {:>12}  {}\n",
            "View Key".cyan().bold(),
            "AViewKey1cWek62tM1kaHqDqwFBK6dZwYHaKeN8DZ6A82v9AEJiCj"
        );
        expected += &format!(
            " {:>12}  {}",
            "Address".cyan().bold(),
            "aleo17h3hthqzctgc5s847pw7c4zmxqce8dxzsmwr3tp7umxcegl9vgrqz2khpg"
        );
        let account = Account::New { seed };
        let actual = account.parse().unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_new_seeded_with_256bits_input() {
        let seed =
            BigInt::from_str("38868010450269069756484274649022187108349082664538872491798902858296683054657").ok();
        let mut expected = format!(
            " {:>12}  {}\n",
            "Private Key".cyan().bold(),
            "APrivateKey1zkp9hECGeW7orKTdzfeZdY8GSXDbiFQxBZZsJZzeGhDBeUo"
        );
        expected += &format!(
            " {:>12}  {}\n",
            "View Key".cyan().bold(),
            "AViewKey1f1cZnyFi2gtPrG8veomDBJz5r945LhE5NutnSeVak5wm"
        );
        expected += &format!(
            " {:>12}  {}",
            "Address".cyan().bold(),
            "aleo1wympy5rs4jrhrqe0lmptsst6efv45ec4pwq37xe2h8sg4ask8yystp95vj"
        );
        let account = Account::New { seed };
        let actual = account.parse().unwrap();
        assert_eq!(expected, actual);
    }
}
