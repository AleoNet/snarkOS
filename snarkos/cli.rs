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

use crate::{Account, Node, Updater};
use snarkos_environment::{helpers::NodeType, Client, Environment};
use snarkvm::prelude::{Address, Network, PrivateKey, ViewKey};

use anyhow::{bail, ensure, Result};
use clap::Parser;
use colored::*;
use std::{fmt::Write, net::SocketAddr, str::FromStr};

#[derive(Debug, Parser)]
#[clap(name = "snarkos", author = "The Aleo Team <hello@aleo.org>")]
pub struct CLI {
    /// Specify the network of this node.
    #[clap(default_value = "3", long = "network")]
    pub network: u16,
    /// Specify the account private key of this node.
    #[clap(long = "private_key")]
    pub private_key: Option<String>,
    /// Specify this as a prover node, with the given prover address.
    #[clap(long = "prover")]
    pub prover: Option<String>,
    /// Specify this as a validator node, with the given validator address.
    #[clap(long = "validator")]
    pub validator: Option<String>,
    #[clap(hide = true, long)]
    pub beacon: bool,

    /// Specify the IP address and port for the node server.
    #[clap(default_value = "0.0.0.0:4133", long = "node")]
    pub node: SocketAddr,
    /// Specify the IP address and port of a beacon node to connect to.
    #[clap(hide = true, default_value = "159.203.77.113:4130", long = "connect_to_beacon")]
    pub beacon_addr: SocketAddr,
    /// Specify the IP address and port of a peer to connect to.
    #[clap(long = "connect")]
    pub connect: Option<String>,

    /// Specify the IP address and port for the RPC server.
    #[clap(default_value = "0.0.0.0:3033", long = "rpc")]
    pub rpc: SocketAddr,
    /// Specify the username for the RPC server.
    #[clap(default_value = "root", long = "username")]
    pub rpc_username: String,
    /// Specify the password for the RPC server.
    #[clap(default_value = "pass", long = "password")]
    pub rpc_password: String,
    /// If the flag is set, the node will not initialize the RPC server.
    #[clap(long)]
    pub norpc: bool,

    /// Specify the verbosity of the node [options: 0, 1, 2, 3]
    #[clap(default_value = "2", long = "verbosity")]
    pub verbosity: u8,
    /// Enables development mode, specify a unique ID for the local node.
    #[clap(long)]
    pub dev: Option<u16>,

    /// Specify an optional subcommand.
    #[clap(subcommand)]
    commands: Option<Command>,
}

impl CLI {
    /// Starts the node.
    pub async fn start(self) -> Result<()> {
        // A type for Aleo Testnet3.
        pub type Testnet3 = snarkvm::prelude::Testnet3;

        // Parse optional subcommands first.
        match self.commands {
            Some(command) => {
                println!("{}", command.parse()?);
                Ok(())
            }
            None => match (self.network, self.node_type()?) {
                (3, NodeType::Client) => self.start_node::<Testnet3, Client<Testnet3>>(&None).await,
                (3, NodeType::Prover) => bail!("Provers will be available in Phase 2"),
                (3, NodeType::Validator) => bail!("Validators will be available in Phase 3"),
                (3, NodeType::Beacon) => bail!("Beacons will be available in Phase 3"),
                _ => bail!("Unsupported network"),
            },
        }
    }

    /// Returns the node type corresponding to the given CLI configurations.
    pub fn node_type(&self) -> Result<NodeType> {
        match (&self.prover, &self.validator, self.beacon) {
            (None, None, false) => Ok(NodeType::Client),
            (Some(_), None, false) => Ok(NodeType::Prover),
            (None, Some(_), false) => Ok(NodeType::Validator),
            (None, None, true) => Ok(NodeType::Beacon),
            _ => bail!("Unsupported node type"),
        }
    }

    /// Starts the node server.
    async fn start_node<N: Network, E: Environment>(&self, address: &Option<String>) -> Result<()> {
        // Construct the Aleo account.
        let account = match (&self.private_key, address) {
            (Some(private_key), Some(address)) => {
                // Construct the account from the private key string.
                let account = Account::from_str(private_key)?;
                // Ensure the account address matches the declared address.
                ensure!(
                    account.address() == &Address::from_str(address)?,
                    "Mismatching private key and address"
                );
                // Return the account.
                account
            }
            // Construct the account from the private key string.
            (Some(private_key), None) => Account::from_str(private_key)?,
            // Throw an error, if no private key is provided but an address is given.
            (None, Some(address)) => {
                bail!("Missing a private key (use '--private_key {{PRIVATE_KEY}}') for address {address}")
            }
            // Sample a new account.
            (None, None) => Account::sample()?,
        };

        // Print the welcome.
        println!("{}", crate::logger::welcome_message());
        // Print the Aleo address.
        println!("Your Aleo address is {}.\n", account.address());
        // Print the node type and network.
        println!("Starting {} on {}.\n", E::NODE_TYPE.description(), N::NAME);

        // Initialize the node.
        let node = Node::<N, E>::new(self, account).await?;

        // Initialize signal handling and maintain ownership of the node - to keep it in scope.
        Self::handle_signals(node.clone());

        // Connect to peer(s) if given as an argument.
        if let Some(peer_ips) = &self.connect {
            // Separate the IP addresses.
            for peer_ip in peer_ips.split(',') {
                // Parse each IP address.
                node.connect_to(match peer_ip.parse() {
                    Ok(ip) => ip,
                    Err(e) => {
                        error!("The IP supplied to --connect ('{peer_ip}') is malformed: {e}");
                        continue;
                    }
                })
                .await?;
            }
        }

        // Note: Do not move this. The pending await must be here otherwise
        // other snarkOS commands will not exit.
        std::future::pending::<()>().await;

        Ok(())
    }

    /// Handles OS signals for the node to intercept and perform a clean shutdown.
    /// Note: Only Ctrl-C is supported; it should work on both Unix-family systems and Windows.
    pub fn handle_signals<N: Network, E: Environment>(node: Node<N, E>) {
        E::resources().register_task(
            None, // No need to provide an id, as the task will run indefinitely.
            tokio::task::spawn(async move {
                match tokio::signal::ctrl_c().await {
                    Ok(()) => {
                        node.shut_down().await;
                        std::process::exit(0);
                    }
                    Err(error) => error!("tokio::signal::ctrl_c encountered an error: {}", error),
                }
            }),
        );
    }
}

#[derive(Debug, Parser)]
pub enum Command {
    #[clap(name = "clean", about = "Removes the ledger files from storage")]
    Clean(Clean),
    #[clap(name = "update", about = "Updates snarkOS to the latest version")]
    Update(Update),
    #[clap(name = "experimental", about = "Experimental features")]
    Experimental(Experimental),
}

impl Command {
    pub fn parse(self) -> Result<String> {
        match self {
            Self::Clean(command) => command.parse(),
            Self::Update(command) => command.parse(),
            Self::Experimental(command) => command.parse(),
        }
    }
}

#[derive(Debug, Parser)]
pub struct Clean {
    /// Specify the network of the ledger to remove from storage.
    #[clap(default_value = "3", long = "network")]
    pub network: u16,
    /// Enables development mode, specify the unique ID of the local node to clean.
    #[clap(long)]
    pub dev: Option<u16>,
}

impl Clean {
    pub fn parse(self) -> Result<String> {
        // Remove the specified ledger from storage.
        Self::remove_ledger(self.network, self.dev)
    }

    /// Removes the specified ledger from storage.
    fn remove_ledger(network: u16, dev: Option<u16>) -> Result<String> {
        // Construct the path to the ledger in storage.
        let path = aleo_std::aleo_ledger_dir(network, dev);
        // Check if the path to the ledger exists in storage.
        if path.exists() {
            // Remove the ledger files from storage.
            match std::fs::remove_dir_all(&path) {
                Ok(_) => Ok(format!("Successfully removed the ledger files from storage. ({})", path.display())),
                Err(error) => bail!("Failed to remove the ledger files from storage. ({})\n{}", path.display(), error),
            }
        } else {
            Ok(format!("No ledger files were found in storage. ({})", path.display()))
        }
    }
}

#[derive(Debug, Parser)]
pub struct Update {
    /// Lists all available versions of snarkOS
    #[clap(short = 'l', long)]
    list: bool,
    /// Suppress outputs to terminal
    #[clap(short = 'q', long)]
    quiet: bool,
    /// Update to specified version
    #[clap(short = 'v', long)]
    version: Option<String>,
}

impl Update {
    pub fn parse(self) -> Result<String> {
        match self.list {
            true => match Updater::show_available_releases() {
                Ok(output) => Ok(output),
                Err(error) => Ok(format!("Failed to list the available versions of snarkOS\n{}\n", error)),
            },
            false => {
                let result = Updater::update_to_release(!self.quiet, self.version);
                if !self.quiet {
                    match result {
                        Ok(status) => {
                            if status.uptodate() {
                                Ok("\nsnarkOS is already on the latest version".to_string())
                            } else if status.updated() {
                                Ok(format!("\nsnarkOS has updated to version {}", status.version()))
                            } else {
                                Ok(String::new())
                            }
                        }
                        Err(e) => Ok(format!("\nFailed to update snarkOS to the latest version\n{}\n", e)),
                    }
                } else {
                    Ok(String::new())
                }
            }
        }
    }
}

#[derive(Debug, Parser)]
pub struct Experimental {
    #[clap(subcommand)]
    commands: ExperimentalCommands,
}

impl Experimental {
    pub fn parse(self) -> Result<String> {
        match self.commands {
            ExperimentalCommands::NewAccount(command) => command.parse(),
        }
    }
}

#[derive(Debug, Parser)]
pub enum ExperimentalCommands {
    #[clap(name = "new_account", about = "Generate a new Aleo account.")]
    NewAccount(NewAccount),
}

#[derive(Debug, Parser)]
pub struct NewAccount {}

impl NewAccount {
    pub fn parse(self) -> Result<String> {
        // Sample a new private key, view key, and address.
        let private_key = PrivateKey::<snarkvm::prelude::Testnet3>::new(&mut rand::thread_rng())?;
        let view_key = ViewKey::try_from(&private_key)?;
        let address = Address::try_from(&view_key)?;

        // Print the new Aleo account.
        let mut output = "".to_string();
        write!(
            output,
            "\n {:>12}\n",
            "Attention - Remember to store this account private key and view key.".red().bold()
        )?;
        writeln!(output, "\n {:>12}  {}", "Private Key".cyan().bold(), private_key)?;
        writeln!(output, " {:>12}  {}", "View Key".cyan().bold(), view_key)?;
        writeln!(output, " {:>12}  {}", "Address".cyan().bold(), address)?;

        Ok(output)
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
