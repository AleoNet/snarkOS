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

use crate::{
    environment::{Client, ClientTrial, Environment, Miner, MinerTrial, NodeType, Operator, OperatorTrial, Prover, ProverTrial, SyncNode},
    helpers::{Tasks, Updater},
    network::Server,
    Display,
};
use snarkos_storage::storage::rocksdb::RocksDB;
use snarkvm::dpc::{prelude::*, testnet2::Testnet2};

use anyhow::{anyhow, Result};
use colored::*;
use crossterm::tty::IsTty;
use std::{io, net::SocketAddr, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use tokio::{signal, sync::mpsc, task};
use tracing_subscriber::EnvFilter;

#[derive(StructOpt, Debug)]
#[structopt(name = "snarkos", author = "The Aleo Team <hello@aleo.org>", setting = structopt::clap::AppSettings::ColoredHelp)]
pub struct Node {
    /// Specify the IP address and port of a peer to connect to.
    #[structopt(long = "connect")]
    pub connect: Option<String>,
    /// Specify this as a mining node, with the given miner address.
    #[structopt(long = "miner")]
    pub miner: Option<String>,
    /// Specify this as an operating node, with the given operator address.
    #[structopt(long = "operator")]
    pub operator: Option<String>,
    /// Specify this as a prover node, with the given prover address.
    #[structopt(long = "prover")]
    pub prover: Option<String>,
    /// Specify the pool IP address.
    /// TODO (howardwu): Remedy how pool IPs are defined.
    #[structopt(long = "pool_ip")]
    pub pool: Option<SocketAddr>,
    /// Specify the network of this node.
    #[structopt(default_value = "2", long = "network")]
    pub network: u16,
    /// Specify the IP address and port for the node server.
    #[structopt(parse(try_from_str), default_value = "0.0.0.0:4132", long = "node")]
    pub node: SocketAddr,
    /// Specify the IP address and port for the RPC server.
    #[structopt(parse(try_from_str), default_value = "0.0.0.0:3032", long = "rpc")]
    pub rpc: SocketAddr,
    /// Specify the username for the RPC server.
    #[structopt(default_value = "root", long = "username")]
    pub rpc_username: String,
    /// Specify the password for the RPC server.
    #[structopt(default_value = "pass", long = "password")]
    pub rpc_password: String,
    /// Specify the verbosity of the node [options: 0, 1, 2, 3]
    #[structopt(default_value = "2", long = "verbosity")]
    pub verbosity: u8,
    /// Enables development mode, specify a unique ID for the local node.
    #[structopt(long)]
    pub dev: Option<u16>,
    /// If the flag is set, the node will render a read-only display.
    #[structopt(long)]
    pub display: bool,
    /// If the flag is set, the node will not initialize the RPC server.
    #[structopt(long)]
    pub norpc: bool,
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
                println!("{}", command.parse()?);
                Ok(())
            }
            None => match (self.network, &self.miner, &self.operator, &self.prover, self.trial, self.sync) {
                (2, None, None, None, false, false) => self.start_server::<Testnet2, Client<Testnet2>>(&None).await,
                (2, Some(_), None, None, false, false) => self.start_server::<Testnet2, Miner<Testnet2>>(&self.miner).await,
                (2, None, Some(_), None, false, false) => self.start_server::<Testnet2, Operator<Testnet2>>(&self.operator).await,
                (2, None, None, Some(_), false, false) => self.start_server::<Testnet2, Prover<Testnet2>>(&self.prover).await,
                (2, None, None, None, true, false) => self.start_server::<Testnet2, ClientTrial<Testnet2>>(&None).await,
                (2, Some(_), None, None, true, false) => self.start_server::<Testnet2, MinerTrial<Testnet2>>(&self.miner).await,
                (2, None, Some(_), None, true, false) => self.start_server::<Testnet2, OperatorTrial<Testnet2>>(&self.operator).await,
                (2, None, None, Some(_), true, false) => self.start_server::<Testnet2, ProverTrial<Testnet2>>(&self.prover).await,
                (2, None, None, None, _, true) => self.start_server::<Testnet2, SyncNode<Testnet2>>(&None).await,
                _ => panic!("Unsupported node configuration"),
            },
        }
    }

    /// Returns the storage path of the ledger.
    pub(crate) fn ledger_storage_path(&self, _local_ip: SocketAddr) -> PathBuf {
        cfg_if::cfg_if! {
            if #[cfg(feature = "test")] {
                // Tests may use any available ports, and removes the storage artifacts afterwards,
                // so that there is no need to adhere to a specific number assignment logic.
                PathBuf::from(format!("/tmp/snarkos-test-ledger-{}", _local_ip.port()))
            } else {
                aleo_std::aleo_ledger_dir(self.network, self.dev)
            }
        }
    }

    /// Returns the storage path of the operator.
    pub(crate) fn operator_storage_path(&self, _local_ip: SocketAddr) -> PathBuf {
        cfg_if::cfg_if! {
            if #[cfg(feature = "test")] {
                // Tests may use any available ports, and removes the storage artifacts afterwards,
                // so that there is no need to adhere to a specific number assignment logic.
                PathBuf::from(format!("/tmp/snarkos-test-operator-{}", _local_ip.port()))
            } else {
                aleo_std::aleo_operator_dir(self.network, self.dev)
            }
        }
    }

    /// Returns the storage path of the prover.
    pub(crate) fn prover_storage_path(&self, _local_ip: SocketAddr) -> PathBuf {
        cfg_if::cfg_if! {
            if #[cfg(feature = "test")] {
                // Tests may use any available ports, and removes the storage artifacts afterwards,
                // so that there is no need to adhere to a specific number assignment logic.
                PathBuf::from(format!("/tmp/snarkos-test-prover-{}", _local_ip.port()))
            } else {
                aleo_std::aleo_prover_dir(self.network, self.dev)
            }
        }
    }

    async fn start_server<N: Network, E: Environment>(&self, address: &Option<String>) -> Result<()> {
        println!("{}", crate::display::welcome_message());

        let address = match (E::NODE_TYPE, address) {
            (NodeType::Miner, Some(address)) | (NodeType::Operator, Some(address)) | (NodeType::Prover, Some(address)) => {
                let address = Address::<N>::from_str(address)?;
                println!("Your Aleo address is {}.\n", address);
                Some(address)
            }
            _ => None,
        };

        println!("Starting {} on {}.", E::NODE_TYPE.description(), N::NETWORK_NAME);
        println!("{}", crate::display::notification_message::<N>(address));

        // Initialize the tasks handler.
        let tasks = Tasks::new();

        // Initialize the node's server.
        let server = Server::<N, E>::initialize(self, address, self.pool, tasks.clone()).await?;

        // Initialize signal handling; it also maintains ownership of the Server
        // in order for it to not go out of scope.
        let server_clone = server.clone();
        handle_signals(server_clone);

        // Initialize the display, if enabled.
        if self.display {
            println!("\nThe snarkOS console is initializing...\n");
            let _display = Display::<N, E>::start(server.clone(), self.verbosity)?;
        };

        // Connect to a peer if one was given as an argument.
        if let Some(peer_ip) = &self.connect {
            let _ = server.connect_to(peer_ip.parse().unwrap()).await;
        }

        // Note: Do not move this. The pending await must be here otherwise
        // other snarkOS commands will not exit.
        std::future::pending::<()>().await;

        Ok(())
    }
}

pub fn initialize_logger(verbosity: u8, log_sender: Option<mpsc::Sender<Vec<u8>>>) {
    match verbosity {
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
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(log_sender.is_none() && io::stdout().is_tty())
        .with_writer(move || LogWriter::new(&log_sender))
        .with_target(verbosity == 3)
        .try_init();
}

#[derive(StructOpt, Debug)]
pub enum Command {
    #[structopt(name = "clean", about = "Removes the ledger files from storage")]
    Clean(Clean),
    #[structopt(name = "update", about = "Updates snarkOS to the latest version")]
    Update(Update),
    #[structopt(name = "experimental", about = "Experimental features")]
    Experimental(Experimental),
    #[structopt(name = "miner", about = "Miner commands and settings")]
    Miner(MinerSubcommand),
}

impl Command {
    pub fn parse(self) -> Result<String> {
        match self {
            Self::Clean(command) => command.parse(),
            Self::Update(command) => command.parse(),
            Self::Experimental(command) => command.parse(),
            Self::Miner(command) => command.parse(),
        }
    }
}

enum LogWriter {
    Stdout(io::Stdout),
    Sender(mpsc::Sender<Vec<u8>>),
}

impl LogWriter {
    fn new(log_sender: &Option<mpsc::Sender<Vec<u8>>>) -> Self {
        if let Some(sender) = log_sender {
            Self::Sender(sender.clone())
        } else {
            Self::Stdout(io::stdout())
        }
    }
}

impl io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Stdout(stdout) => stdout.write(buf),
            Self::Sender(sender) => {
                let log = buf.to_vec();
                let _ = sender.try_send(log);
                Ok(buf.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(StructOpt, Debug)]
pub struct Clean {
    /// Specify the network of the ledger to remove from storage.
    #[structopt(default_value = "2", long = "network")]
    pub network: u16,
    /// Enables development mode, specify the unique ID of the local node to clean.
    #[structopt(long)]
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
                Err(error) => Err(anyhow!(
                    "Failed to remove the ledger files from storage. ({})\n{}",
                    path.display(),
                    error
                )),
            }
        } else {
            Ok(format!("No ledger files were found in storage. ({})", path.display()))
        }
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
    /// Update to specified version
    #[structopt(short = "v", long)]
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
    #[structopt(name = "new_account", about = "Generate a new Aleo account.")]
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

#[derive(StructOpt, Debug)]
pub struct MinerSubcommand {
    #[structopt(subcommand)]
    commands: MinerCommands,
}

impl MinerSubcommand {
    pub fn parse(self) -> Result<String> {
        match self.commands {
            MinerCommands::Stats(command) => command.parse(),
        }
    }
}

#[derive(StructOpt, Debug)]
pub enum MinerCommands {
    #[structopt(name = "stats", about = "Prints statistics for the miner.")]
    Stats(MinerStats),
}

#[derive(StructOpt, Debug)]
pub struct MinerStats {
    #[structopt()]
    address: String,
}

impl MinerStats {
    pub fn parse(self) -> Result<String> {
        // Parse the input address.
        let miner = Address::<Testnet2>::from_str(&self.address)?;

        // Initialize the node.
        let node = Node::from_iter(&["snarkos", "--norpc", "--verbosity", "0"]);

        let ip = "0.0.0.0:1000".parse().unwrap();

        // Initialize the ledger storage.
        let ledger_storage_path = node.ledger_storage_path(ip);
        let ledger = snarkos_storage::LedgerState::<Testnet2>::open_reader::<RocksDB, _>(ledger_storage_path).unwrap();

        // Initialize the prover storage.
        let prover_storage_path = node.prover_storage_path(ip);
        let prover = snarkos_storage::ProverState::<Testnet2>::open_writer::<RocksDB, _>(prover_storage_path).unwrap();

        // Retrieve the latest block height.
        let latest_block_height = ledger.latest_block_height();

        // Prepare a list of confirmed and pending coinbase records.
        let mut confirmed = vec![];
        let mut pending = vec![];

        // Iterate through the coinbase records from storage.
        for (block_height, record) in prover.to_coinbase_records() {
            // Filter the coinbase records by determining if they exist on the canonical chain.
            if let Ok(true) = ledger.contains_commitment(&record.commitment()) {
                // Ensure the record owner matches.
                if record.owner() == miner {
                    // Add the block to the appropriate list.
                    match block_height + 2048 < latest_block_height {
                        true => confirmed.push((block_height, record)),
                        false => pending.push((block_height, record)),
                    }
                }
            }
        }

        return Ok(format!(
            "Mining Report (confirmed_blocks = {}, pending_blocks = {}, miner_address = {})",
            confirmed.len(),
            pending.len(),
            miner
        ));
    }
}

// This function is responsible for handling OS signals in order for the node to be able to intercept them
// and perform a clean shutdown.
// note: only Ctrl-C is currently supported, but it should work on both Unix-family systems and Windows.
fn handle_signals<N: Network, E: Environment>(server: Server<N, E>) {
    task::spawn(async move {
        match signal::ctrl_c().await {
            Ok(()) => {
                server.shut_down().await;
                std::process::exit(0);
            }
            Err(error) => error!("tokio::signal::ctrl_c encountered an error: {}", error),
        }
    });
}
