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

mod account;
pub use account::*;

mod clean;
pub use clean::*;

mod start;
pub use start::*;

mod update;
pub use update::*;

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(name = "snarkOS", author = "The Aleo Team <hello@aleo.org>", setting = clap::AppSettings::ColoredHelp)]
pub struct CLI {
    /// Specify the verbosity [options: 0, 1, 2, 3]
    #[clap(default_value = "2", short, long)]
    pub verbosity: u8,
    /// Specify a subcommand.
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Parser)]
pub enum Command {
    #[clap(subcommand)]
    Account(Account),
    #[clap(name = "clean")]
    Clean(Clean),
    #[clap(name = "start")]
    Start(Box<Start>),
    #[clap(name = "update")]
    Update(Update),
}

impl Command {
    /// Parses the command.
    pub fn parse(self) -> Result<String> {
        match self {
            Self::Account(command) => command.parse(),
            Self::Clean(command) => command.parse(),
            Self::Start(command) => command.parse(),
            Self::Update(command) => command.parse(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // As per the official clap recommendation.
    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        CLI::command().debug_assert()
    }
}
