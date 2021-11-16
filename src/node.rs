// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use crate::{helpers::Updater, network::Server, Client, ClientTrial, Display, Environment, Miner, MinerTrial, NodeType, SyncNode};
use snarkvm::dpc::{prelude::*, testnet2::Testnet2};

use anyhow::Result;
use colored::*;
use std::{fs, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use tracing_subscriber::EnvFilter;

#[derive(StructOpt, Debug)]
#[structopt(name = "snarkos", author = "The Aleo Team <hello@aleo.org>", setting = structopt::clap::AppSettings::ColoredHelp)]
pub struct Node {
    /// Specify the IP address of a peer to connect to.
    #[structopt(long = "connect")]
    pub connect: Option<String>,
    /// Specify this as a mining node, with the given miner address.
    #[structopt(long = "miner")]
    pub miner: Option<String>,
    /// Specify the network of this node.
    #[structopt(default_value = "2", short = "n", long = "network")]
    pub network: u16,
    /// Specify the port for the node server.
    #[structopt(long = "node")]
    pub node: Option<u16>,
    /// Specify the port for the RPC server.
    #[structopt(long = "rpc")]
    pub rpc: Option<u16>,
    /// Specify the username for the RPC server.
    #[structopt(default_value = "root", long = "username")]
    pub rpc_username: String,
    /// Specify the password for the RPC server.
    #[structopt(default_value = "pass", long = "password")]
    pub rpc_password: String,
    /// Specify the verbosity of the node [options: 0, 1, 2, 3]
    #[structopt(default_value = "3", long = "verbosity")]
    pub verbosity: u8,
    /// If the flag is set, the node will render a read-only display.
    #[structopt(long)]
    pub display: bool,
    #[structopt(hidden = true, long)]
    pub trial: bool,
    #[structopt(hidden = true, long)]
    pub sync: bool,
    /// Specify an optional subcommand.
    #[structopt(subcommand)]
    commands: Option<Command>,
}

impl Node {
    /// Starts the node.
    pub async fn start(self) -> Result<()> {
        // Parse optional subcommands first.
        match self.commands {
            Some(command) => {
                println!("{}", command.parse::<Testnet2>()?);
                Ok(())
            }
            None => match (self.network, self.miner.is_some(), self.trial, self.sync) {
                (2, _, _, true) => self.start_server::<Testnet2, SyncNode<Testnet2>>().await,
                (2, true, false, false) => self.start_server::<Testnet2, Miner<Testnet2>>().await,
                (2, false, false, false) => self.start_server::<Testnet2, Client<Testnet2>>().await,
                (2, true, true, false) => self.start_server::<Testnet2, MinerTrial<Testnet2>>().await,
                (2, false, true, false) => self.start_server::<Testnet2, ClientTrial<Testnet2>>().await,
                _ => panic!("Unsupported node configuration"),
            },
        }
    }

    async fn start_server<N: Network, E: Environment>(&self) -> Result<()> {
        let node_port = self.node.unwrap_or(E::DEFAULT_NODE_PORT);
        let rpc_port = self.rpc.unwrap_or(E::DEFAULT_RPC_PORT);
        assert!(
            !(node_port < 4130),
            "Until configuration files are established, the node port must be at least 4130 or greater"
        );

        let miner = match (E::NODE_TYPE, &self.miner) {
            (NodeType::Miner, Some(address)) => {
                let miner_address = Address::<N>::from_str(address)?;
                println!("{}", crate::display::welcome_message());
                println!("Your Aleo address is {}.\n", miner_address);
                println!("Starting a mining node on {}.\n", N::NETWORK_NAME);
                Some(miner_address)
            }
            _ => {
                println!("{}", crate::display::welcome_message());
                println!("Starting a client node on {}.\n", N::NETWORK_NAME);
                None
            }
        };

        if self.display {
            println!("\nThe snarkOS console is initializing...\n");
            let server =
                Server::<N, E>::initialize(node_port, rpc_port, self.rpc_username.clone(), self.rpc_password.clone(), miner).await?;
            if let Some(peer_ip) = &self.connect {
                server.connect_to(peer_ip.parse().unwrap()).await?;
            }
            let _display = Display::<N, E>::start(server)?;
            Ok(())
        } else {
            self.initialize_logger();
            let server =
                Server::<N, E>::initialize(node_port, rpc_port, self.rpc_username.clone(), self.rpc_password.clone(), miner).await?;
            if let Some(peer_ip) = &self.connect {
                server.connect_to(peer_ip.parse().unwrap()).await?;
            }
            std::future::pending::<()>().await;
            Ok(())
        }
    }

    fn initialize_logger(&self) {
        match self.verbosity {
            0 => std::env::set_var("RUST_LOG", "info"),
            1 => std::env::set_var("RUST_LOG", "debug"),
            2 | 3 => std::env::set_var("RUST_LOG", "trace"),
            _ => std::env::set_var("RUST_LOG", "info"),
        };

        // Filter out undesirable logs.
        let filter = EnvFilter::from_default_env()
            .add_directive("mio=off".parse().unwrap())
            .add_directive("tokio_util=off".parse().unwrap())
            .add_directive("hyper::proto::h1::conn=off".parse().unwrap())
            .add_directive("hyper::proto::h1::decode=off".parse().unwrap())
            .add_directive("hyper::proto::h1::io=off".parse().unwrap())
            .add_directive("hyper::proto::h1::role=off".parse().unwrap());

        // Initialize tracing.
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(self.verbosity == 3)
            .init();
    }
}

#[derive(StructOpt, Debug)]
pub enum Command {
    #[structopt(name = "clean", about = "Removes ledger storage files")]
    Clean(Clean),
    #[structopt(name = "update", about = "Updates snarkOS to the latest version")]
    Update(Update),
    #[structopt(name = "experimental", about = "Experimental features")]
    Experimental(Experimental),
}

impl Command {
    pub fn parse<N: Network>(self) -> Result<String> {
        match self {
            Self::Clean(command) => command.parse::<N>(),
            Self::Update(command) => command.parse(),
            Self::Experimental(command) => command.parse(),
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct Clean {
    /// The ledger storage number (.ledger-[number]) to remove. Removes all storage by default.
    #[structopt(short, long)]
    number: Option<u8>,
}

impl Clean {
    /// Returns the result of executing the snarkos clean command.
    pub fn parse<N: Network>(self) -> Result<String> {
        // Compute the path to the aleo directory.
        let mut path = aleo_std::aleo_dir();
        path.push(N::NETWORK_NAME);

        // Check if the aleo directory exists.
        if !path.exists() {
            return Ok("Aleo directory not found. No ledger storage files to remove".to_string());
        }

        // Remove one or more ledger storage directories.
        match self.number {
            Some(number) => Self::remove_ledger(path, number),
            None => Self::remove_all_ledgers(path),
        }
    }

    /// Removes a single ledger storage directory at `ledger-[number]` in the given path.
    fn remove_ledger(mut path: PathBuf, number: u8) -> Result<String> {
        // Compute the path to the ledger storage directory.
        path.push(format!("ledger-{}", number));

        // Check if the directory exists.
        if !path.exists() {
            return Ok(format!("No ledger storage files found at {}", path.display()));
        }

        // Remove the directory.
        println!("Removing {}", path.display());
        fs::remove_dir_all(path)?;

        Ok("Successfully removed ledger storage files".to_string())
    }

    /// Removes all ledger storage directories in the given path.
    fn remove_all_ledgers(path: PathBuf) -> Result<String> {
        // Filter for ledger storage directories.
        let ledgers = fs::read_dir(path)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_type().expect("Unable to read file type").is_dir() && entry.path().display().to_string().contains("ledger-")
            })
            .collect::<Vec<_>>();

        // Check if there are any directories to remove.
        if ledgers.len() == 0 {
            return Ok("No ledger storage files to remove".to_string());
        }

        // Remove the directories.
        for entry in ledgers {
            let path = entry.path();
            println!("Removing {}", path.display());

            fs::remove_dir_all(path)?;
        }

        Ok("Successfully removed all ledger storage files".to_string())
    }
}

#[derive(StructOpt, Debug)]
pub struct Update {
    /// Lists all available versions of snarkOS
    #[structopt(short = "l", long)]
    list: bool,

    /// Suppress outputs to terminal
    #[structopt(short = "q", long)]
    quiet: bool,
}

impl Update {
    pub fn parse(self) -> Result<String> {
        match self.list {
            true => match Updater::show_available_releases() {
                Ok(output) => Ok(output),
                Err(error) => Ok(format!("Failed to list the available versions of snarkOS\n{}\n", error)),
            },
            false => {
                let result = Updater::update_to_latest_release(!self.quiet);
                if !self.quiet {
                    match result {
                        Ok(status) => {
                            if status.uptodate() {
                                Ok("\nsnarkOS is already on the latest version".to_string())
                            } else if status.updated() {
                                Ok(format!("\nsnarkOS has updated to version {}", status.version()))
                            } else {
                                Ok(format!(""))
                            }
                        }
                        Err(e) => Ok(format!("\nFailed to update snarkOS to the latest version\n{}\n", e)),
                    }
                } else {
                    Ok(format!(""))
                }
            }
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct Experimental {
    #[structopt(subcommand)]
    commands: ExperimentalCommands,
}

impl Experimental {
    pub fn parse(self) -> Result<String> {
        match self.commands {
            ExperimentalCommands::NewAccount(command) => command.parse(),
        }
    }
}

#[derive(StructOpt, Debug)]
pub enum ExperimentalCommands {
    #[structopt(name = "new_account", about = "Generate a new Aleo Account.")]
    NewAccount(NewAccount),
}

#[derive(StructOpt, Debug)]
pub struct NewAccount {}

impl NewAccount {
    pub fn parse(self) -> Result<String> {
        let account = Account::<Testnet2>::new(&mut rand::thread_rng());

        // Print the new Aleo account.
        let mut output = "".to_string();
        output += &format!(
            "\n {:>12}\n",
            "Attention - Remember to store this account private key and view key.".red().bold()
        );
        output += &format!("\n {:>12}  {}\n", "Private Key".cyan().bold(), account.private_key());
        output += &format!(" {:>12}  {}\n", "View Key".cyan().bold(), account.view_key());
        output += &format!(" {:>12}  {}\n", "Address".cyan().bold(), account.address());

        Ok(output)
    }
}
