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

use snarkos_cli::{commands::CLI, helpers::Updater};

use clap::Parser;

fn main() -> anyhow::Result<()> {
    // Parse the given arguments.
    let cli = CLI::parse();
    // Run the updater.
    println!("{}", Updater::print_cli());
    // Run the CLI.
    match cli.command.parse() {
        Ok(output) => println!("{output}\n"),
        Err(error) => println!("⚠️  {error}\n"),
    }
    Ok(())
}
