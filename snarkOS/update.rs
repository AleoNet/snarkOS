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

        println!("List of available snarkOS release verions");
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
