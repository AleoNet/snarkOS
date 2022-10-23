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

use snarkvm::prelude::PrivateKey;

use anyhow::Result;
use clap::Parser;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;

type Network = snarkvm::prelude::Testnet3;

/// Commands to manage Aleo accounts.
#[derive(Debug, Parser)]
pub enum Account {
    /// Generates a new Aleo account
    New {
        /// Seed the RNG with a numeric value
        #[clap(short = 's', long)]
        seed: Option<u64>,
    },
}

impl Account {
    pub fn parse(self) -> Result<String> {
        match self {
            Self::New { seed } => {
                // Sample a new Aleo account.
                let account = snarkos_account::Account::from(match seed {
                    Some(seed) => PrivateKey::<Network>::new(&mut ChaChaRng::seed_from_u64(seed))?,
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

    #[test]
    fn test_new() {
        for _ in 0..3 {
            let account = Account::New { seed: None };
            assert!(account.parse().is_ok());
        }
    }

    #[test]
    fn test_new_seeded() {
        let seed = Some(1231275789u64);
        let mut expected = format!(
            " {:>12}  {}\n",
            "Private Key".cyan().bold(),
            "APrivateKey1zkp2oVPTci9kKcUprnbzMwq95Di1MQERpYBhEeqvkrDirK1"
        );
        expected += &format!(
            " {:>12}  {}\n",
            "View Key".cyan().bold(),
            "AViewKey1mmLWAuYDaM1NfgNaD1Jy7THG8uS4Ui2zyugFuPEijgyQ"
        );
        expected += &format!(
            " {:>12}  {}",
            "Address".cyan().bold(),
            "aleo1whnlxsgnhc8ywft2l4nu9hywedspcjpwcsgg490ckz34tthqsupqdh5z64"
        );
        let account = Account::New { seed };
        let actual = account.parse().unwrap();
        assert_eq!(expected, actual);
    }
}
