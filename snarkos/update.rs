// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use snarkos_errors::node::CliError;

use clap::ArgMatches;
use self_update::{backends::github, cargo_crate_version};

const SNARKOS_BIN_NAME: &str = "snarkos";
const SNARKOS_REPO_OWNER: &str = "AleoHQ";
const SNARKOS_REPO_NAME: &str = "snarkOS";

// TODO formalize the UPDATE CLI Subcommand model
pub struct UpdateCLI;

impl UpdateCLI {
    /// Show all available releases for snarkOS
    fn show_available_releases() -> Result<(), self_update::errors::Error> {
        let releases = github::ReleaseList::configure()
            .repo_owner(SNARKOS_REPO_OWNER)
            .repo_name(SNARKOS_REPO_NAME)
            .build()?
            .fetch()?;

        println!("List of available snarkOS release versions");
        for release in releases {
            println!("* {}", release.version);
        }
        Ok(())
    }

    /// Update to the latest snarkOS release
    fn update_to_latest_release() -> Result<(), self_update::errors::Error> {
        let status = github::Update::configure()
            .repo_owner(SNARKOS_REPO_OWNER)
            .repo_name(SNARKOS_REPO_NAME)
            .bin_name(SNARKOS_BIN_NAME)
            .show_download_progress(true)
            .no_confirm(false)
            .show_output(true)
            .current_version(cargo_crate_version!())
            .build()?
            .update()?;

        println!("snarkOS has successfully updated to version {}", status.version());
        Ok(())
    }

    pub fn parse(arguments: &ArgMatches) -> Result<(), CliError> {
        if arguments.is_present("list") {
            if let Err(e) = Self::show_available_releases() {
                println!("Could not get snarkOS versions");
                println!("Error: {}", e);
            }
        } else {
            if let Err(e) = Self::update_to_latest_release() {
                println!("Could not update snarkOS");
                println!("Error: {}", e);
            }
        }

        Ok(())
    }
}
