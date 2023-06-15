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

use snarkos_account::Account;
use snarkos_display::Display;
use snarkos_node::{messages::NodeType, Node};
use snarkvm::{
    prelude::{Block, ConsensusStore, FromBytes, Network, PrivateKey, Testnet3, VM},
    synthesizer::store::helpers::memory::ConsensusMemory,
};

use anyhow::{bail, Result};
use clap::Parser;
use colored::Colorize;
use core::str::FromStr;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use std::{net::SocketAddr, path::PathBuf};
use tokio::runtime::{self, Runtime};

/// The recommended minimum number of 'open files' limit for a beacon.
/// Beacons should be able to handle at least 1000 concurrent connections, each requiring 2 sockets.
#[cfg(target_family = "unix")]
const RECOMMENDED_MIN_NOFILES_LIMIT_BEACON: u64 = 2048;
/// The recommended minimum number of 'open files' limit for a validator.
/// Validators should be able to handle at least 500 concurrent connections, each requiring 2 sockets.
#[cfg(target_family = "unix")]
const RECOMMENDED_MIN_NOFILES_LIMIT_VALIDATOR: u64 = 1024;

/// Starts the snarkOS node.
#[derive(Clone, Debug, Parser)]
pub struct Start {
    /// Specify the network ID of this node
    #[clap(default_value = "3", long = "network")]
    pub network: u16,

    /// Specify this node as a beacon
    #[clap(long = "beacon")]
    pub beacon: bool,
    /// Specify this node as a validator
    #[clap(long = "validator")]
    pub validator: bool,
    /// Specify this node as a prover
    #[clap(long = "prover")]
    pub prover: bool,
    /// Specify this node as a client
    #[clap(long = "client")]
    pub client: bool,

    /// Specify the node's account private key.
    #[clap(long = "private-key")]
    pub private_key: Option<String>,
    /// Specify the path to the node's account private key.
    #[clap(long = "private-key-path")]
    pub private_key_path: Option<PathBuf>,

    /// Specify the IP address and port for the node server
    #[clap(default_value = "0.0.0.0:4133", long = "node")]
    pub node: SocketAddr,
    /// Specify the IP address and port of a peer to connect to
    #[clap(default_value = "", long = "connect")]
    pub connect: String,

    /// Specify the IP address and port for the REST server
    #[clap(default_value = "0.0.0.0:3033", long = "rest")]
    pub rest: SocketAddr,
    /// If the flag is set, the node will not initialize the REST server
    #[clap(long)]
    pub norest: bool,

    /// If the flag is set, the node will not render the display
    #[clap(long)]
    pub nodisplay: bool,
    /// Specify the verbosity of the node [options: 0, 1, 2, 3, 4]
    #[clap(default_value = "2", long = "verbosity")]
    pub verbosity: u8,
    /// Specify the path to the file where logs will be stored
    #[clap(default_value_os_t = std::env::temp_dir().join("snarkos.log"), long = "logfile")]
    pub logfile: PathBuf,

    /// Enables the node to prefetch initial blocks from a CDN
    #[clap(default_value = "https://testnet3.blocks.aleo.org/phase3", long = "cdn")]
    pub cdn: String,
    /// Enables development mode, specify a unique ID for this node
    #[clap(long)]
    pub dev: Option<u16>,
}

impl Start {
    /// Starts the snarkOS node.
    pub fn parse(self) -> Result<String> {
        // Initialize the logger.
        let log_receiver = crate::helpers::initialize_logger(self.verbosity, self.nodisplay, self.logfile.clone());
        // Initialize the runtime.
        Self::runtime().block_on(async move {
            // Clone the configurations.
            let mut cli = self.clone();
            // Parse the network.
            match cli.network {
                3 => {
                    // Parse the node from the configurations.
                    let node = cli.parse_node::<Testnet3>().await.expect("Failed to parse the node");
                    // If the display is enabled, render the display.
                    if !cli.nodisplay {
                        // Initialize the display.
                        Display::start(node, log_receiver).expect("Failed to initialize the display");
                    }
                }
                _ => panic!("Invalid network ID specified"),
            };
            // Note: Do not move this. The pending await must be here otherwise
            // other snarkOS commands will not exit.
            std::future::pending::<()>().await;
        });

        Ok(String::new())
    }
}

impl Start {
    /// Returns the initial node(s) to connect to, from the given configurations.
    fn parse_trusted_peers(&self) -> Result<Vec<SocketAddr>> {
        match self.connect.is_empty() {
            true => Ok(vec![]),
            false => Ok(self
                .connect
                .split(',')
                .flat_map(|ip| match ip.parse::<SocketAddr>() {
                    Ok(ip) => Some(ip),
                    Err(e) => {
                        eprintln!("The IP supplied to --connect ('{ip}') is malformed: {e}");
                        None
                    }
                })
                .collect()),
        }
    }

    /// Returns the CDN to prefetch initial blocks from, from the given configurations.
    fn parse_cdn(&self) -> Option<String> {
        // Disable CDN if:
        //  1. The node is in development mode.
        //  2. The user has explicitly disabled CDN.
        //  3. The node is a client (no need to sync).
        //  4. The node is a prover (no need to sync).
        if self.dev.is_some() || self.cdn.is_empty() || self.client || self.prover {
            None
        }
        // Check for an edge case, where the node defaults to a client.
        else if !(self.beacon || self.validator || self.prover || self.client) {
            None
        }
        // Enable the CDN otherwise.
        else {
            Some(self.cdn.clone())
        }
    }

    /// Read the private key directly from an argument or from a filesystem location.
    fn parse_private_key(&mut self) -> Result<()> {
        // If the private key is provided directly, don't do anything else.
        if self.private_key.is_some() {
            return Ok(());
        }

        // If a filesystem path to the private key is provided, attempt to
        // read it and overwrite `self.private_key` in case of success.
        if let Some(path) = &self.private_key_path {
            let private_key = std::fs::read_to_string(path)?;
            self.private_key = Some(private_key);
            return Ok(());
        }

        // Only allow the private key to be missing if the node type is unspecified, in line with `parse_account`.
        if self.beacon || self.validator || self.prover || self.client {
            bail!("Misconfiguration; if the node type is provided, the private key must also be present.");
        } else {
            Ok(())
        }
    }

    /// Updates the configurations if the node is in development mode, and returns the
    /// alternative genesis block if the node is in development mode. Otherwise, returns the actual genesis block.
    fn parse_development<N: Network>(&mut self, trusted_peers: &mut Vec<SocketAddr>) -> Result<Block<N>> {
        // If `--dev` is set, assume the dev nodes are initialized from 0 to `dev`,
        // and add each of them to the trusted peers. In addition, set the node IP to `4130 + dev`,
        // and the REST IP to `3030 + dev`.
        if let Some(dev) = self.dev {
            // Only one beacon node is allowed in testing. To avoid ambiguity, we require
            // the beacon to be the first node in the dev network.
            if dev > 0 && self.beacon {
                bail!("At most one beacon at '--dev 0' is supported in development mode");
            }

            // Add the dev nodes to the trusted peers.
            for i in 0..dev {
                trusted_peers.push(SocketAddr::from_str(&format!("127.0.0.1:{}", 4130 + i))?);
            }
            // Set the node IP to `4130 + dev`.
            self.node = SocketAddr::from_str(&format!("0.0.0.0:{}", 4130 + dev))?;
            // Set the REST IP to `3030 + dev`.
            if !self.norest {
                self.rest = SocketAddr::from_str(&format!("0.0.0.0:{}", 3030 + dev))?;
            }

            // Initialize an (insecure) fixed RNG.
            let mut rng = ChaChaRng::seed_from_u64(1234567890u64);
            // Initialize the beacon private key.
            let beacon_private_key = PrivateKey::<N>::new(&mut rng)?;
            // Initialize a new VM.
            let vm = VM::from(ConsensusStore::<N, ConsensusMemory<N>>::open(None)?)?;
            // Initialize the genesis block.
            let genesis = vm.genesis(&beacon_private_key, &mut rng)?;

            // A helper method to set the account private key in the node type.
            let sample_account = |node: &mut Option<String>, is_beacon: bool| -> Result<()> {
                let account = match is_beacon {
                    true => Account::<N>::try_from(beacon_private_key)?,
                    false => Account::<N>::new(&mut rand::thread_rng())?,
                };
                *node = Some(account.private_key().to_string());
                println!(
                    "⚠️  Attention - Sampling a *one-time* account for this instance, please save this securely:\n\n{account}\n"
                );
                Ok(())
            };

            // If the beacon type flag is set, override the private key.
            if self.beacon {
                sample_account(&mut self.private_key, true)?;
            }
            // If the node type flag is set, but no private key is provided, then sample one.
            else if self.private_key.is_none() && (self.validator || self.prover || self.client) {
                sample_account(&mut self.private_key, false)?;
            }

            Ok(genesis)
        } else {
            Block::from_bytes_le(N::genesis_bytes())
        }
    }

    /// Returns the node account and node type, from the given configurations.
    fn parse_account<N: Network>(&self) -> Result<(Account<N>, NodeType)> {
        // `parse_private_key` ensured that the private key, if provided, is stored in `self.private_key`.
        let private_key = self.private_key.as_deref().unwrap_or_default();
        // Ensures only one of the four flags is set. If no flags are set, defaults to a client node.
        match (&self.beacon, &self.validator, &self.prover, &self.client) {
            (true, false, false, false) => Ok((Account::<N>::from_str(private_key)?, NodeType::Beacon)),
            (false, true, false, false) => Ok((Account::<N>::from_str(private_key)?, NodeType::Validator)),
            (false, false, true, false) => Ok((Account::<N>::from_str(private_key)?, NodeType::Prover)),
            (false, false, false, true) => Ok((Account::<N>::from_str(private_key)?, NodeType::Client)),
            (false, false, false, false) => Ok((Account::<N>::new(&mut rand::thread_rng())?, NodeType::Client)),
            _ => bail!("Unsupported node configuration"),
        }
    }

    /// Returns the node type corresponding to the given configurations.
    #[rustfmt::skip]
    async fn parse_node<N: Network>(&mut self) -> Result<Node<N>> {
        // Print the welcome.
        println!("{}", crate::helpers::welcome_message());

        // Parse the trusted IPs to connect to.
        let mut trusted_peers = self.parse_trusted_peers()?;

        // Parse the CDN.
        let cdn = self.parse_cdn();

        // Parse the development configurations, and determine the genesis block.
        let genesis = self.parse_development::<N>(&mut trusted_peers)?;

        // Parse the REST IP.
        let rest_ip = match self.norest {
            true => None,
            false => Some(self.rest),
        };

        // Parse the node's private key.
        self.parse_private_key()?;

        // Parse the node account and node type.
        let (account, node_type) = self.parse_account::<N>()?;

        // If the display is not enabled, render the welcome message.
        if self.nodisplay {
            // Print the Aleo address.
            println!("🪪 Your Aleo address is {}.\n", account.address().to_string().bold());
            // Print the node type and network.
            println!(
                "🧭 Starting {} on {} {} at {}.\n",
                node_type.description().bold(),
                N::NAME.bold(),
                "Phase 3".bold(),
                self.node.to_string().bold()
            );

            // If the node is running a REST server, print the REST IP and JWT.
            if node_type.is_beacon() || node_type.is_validator() {
                if let Some(rest_ip) = rest_ip {
                    println!("🌐 Starting the REST server at {}.\n", rest_ip.to_string().bold());

                    if let Ok(jwt_token) = snarkos_node_rest::Claims::new(account.address()).to_jwt_string() {
                        println!("🔑 Your one-time JWT token is {}\n", jwt_token.dimmed());
                    }
                }
            }
        }

        // If the node is a beacon, check if the open files limit is lower than recommended.
        if node_type.is_beacon() {
            #[cfg(target_family = "unix")]
            crate::helpers::check_open_files_limit(RECOMMENDED_MIN_NOFILES_LIMIT_BEACON);
        }
        // If the node is a validator, check if the open files limit is lower than recommended.
        if node_type.is_validator() {
            #[cfg(target_family = "unix")]
            crate::helpers::check_open_files_limit(RECOMMENDED_MIN_NOFILES_LIMIT_VALIDATOR);
        }

        // Initialize the node.
        match node_type {
            NodeType::Beacon => Node::new_beacon(self.node, rest_ip, account, &trusted_peers, genesis, cdn, self.dev).await,
            NodeType::Validator => Node::new_validator(self.node, rest_ip, account, &trusted_peers, genesis, cdn, self.dev).await,
            NodeType::Prover => Node::new_prover(self.node, account, &trusted_peers, genesis, self.dev).await,
            NodeType::Client => Node::new_client(self.node, account, &trusted_peers, genesis, self.dev).await,
        }
    }

    /// Returns a runtime for the node.
    fn runtime() -> Runtime {
        // TODO (howardwu): Fix this.
        // let (num_tokio_worker_threads, max_tokio_blocking_threads, num_rayon_cores_global) = if !Self::node_type().is_beacon() {
        //     ((num_cpus::get() / 8 * 2).max(1), num_cpus::get(), (num_cpus::get() / 8 * 5).max(1))
        // } else {
        //     (num_cpus::get(), 512, num_cpus::get()) // 512 is tokio's current default
        // };
        let (num_tokio_worker_threads, max_tokio_blocking_threads, num_rayon_cores_global) =
            // { ((num_cpus::get() / 2).max(1), num_cpus::get(), (num_cpus::get() / 4 * 3).max(1)) };
            { (num_cpus::get().min(8), 512, num_cpus::get().saturating_sub(8).max(1)) };

        // Initialize the parallelization parameters.
        rayon::ThreadPoolBuilder::new()
            .stack_size(8 * 1024 * 1024)
            .num_threads(num_rayon_cores_global)
            .build_global()
            .unwrap();

        // Initialize the runtime configuration.
        runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_stack_size(8 * 1024 * 1024)
            .worker_threads(num_tokio_worker_threads)
            .max_blocking_threads(max_tokio_blocking_threads)
            .build()
            .expect("Failed to initialize a runtime for the router")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Command, CLI};
    use snarkvm::prelude::Testnet3;

    type CurrentNetwork = Testnet3;

    #[test]
    fn test_parse_trusted_peers() {
        let config = Start::try_parse_from(["snarkos", "--connect", ""].iter()).unwrap();
        assert!(config.parse_trusted_peers().is_ok());
        assert!(config.parse_trusted_peers().unwrap().is_empty());

        let config = Start::try_parse_from(["snarkos", "--connect", "1.2.3.4:5"].iter()).unwrap();
        assert!(config.parse_trusted_peers().is_ok());
        assert_eq!(config.parse_trusted_peers().unwrap(), vec![SocketAddr::from_str("1.2.3.4:5").unwrap()]);

        let config = Start::try_parse_from(["snarkos", "--connect", "1.2.3.4:5,6.7.8.9:0"].iter()).unwrap();
        assert!(config.parse_trusted_peers().is_ok());
        assert_eq!(config.parse_trusted_peers().unwrap(), vec![
            SocketAddr::from_str("1.2.3.4:5").unwrap(),
            SocketAddr::from_str("6.7.8.9:0").unwrap()
        ]);
    }

    #[test]
    fn test_parse_cdn() {
        // Beacon (Prod)
        let config = Start::try_parse_from(["snarkos", "--beacon", "--private-key", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_some());
        let config =
            Start::try_parse_from(["snarkos", "--beacon", "--private-key", "aleo1xx", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_some());
        let config =
            Start::try_parse_from(["snarkos", "--beacon", "--private-key", "aleo1xx", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());

        // Beacon (Dev)
        let config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--beacon", "--private-key", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(
            ["snarkos", "--dev", "0", "--beacon", "--private-key", "aleo1xx", "--cdn", "url"].iter(),
        )
        .unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(
            ["snarkos", "--dev", "0", "--beacon", "--private-key", "aleo1xx", "--cdn", ""].iter(),
        )
        .unwrap();
        assert!(config.parse_cdn().is_none());

        // Validator (Prod)
        let config = Start::try_parse_from(["snarkos", "--validator", "--private-key", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_some());
        let config =
            Start::try_parse_from(["snarkos", "--validator", "--private-key", "aleo1xx", "--cdn", "url"].iter())
                .unwrap();
        assert!(config.parse_cdn().is_some());
        let config =
            Start::try_parse_from(["snarkos", "--validator", "--private-key", "aleo1xx", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());

        // Validator (Dev)
        let config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--validator", "--private-key", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(
            ["snarkos", "--dev", "0", "--validator", "--private-key", "aleo1xx", "--cdn", "url"].iter(),
        )
        .unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(
            ["snarkos", "--dev", "0", "--validator", "--private-key", "aleo1xx", "--cdn", ""].iter(),
        )
        .unwrap();
        assert!(config.parse_cdn().is_none());

        // Prover (Prod)
        let config = Start::try_parse_from(["snarkos", "--prover", "--private-key", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config =
            Start::try_parse_from(["snarkos", "--prover", "--private-key", "aleo1xx", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config =
            Start::try_parse_from(["snarkos", "--prover", "--private-key", "aleo1xx", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());

        // Prover (Dev)
        let config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--prover", "--private-key", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(
            ["snarkos", "--dev", "0", "--prover", "--private-key", "aleo1xx", "--cdn", "url"].iter(),
        )
        .unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(
            ["snarkos", "--dev", "0", "--prover", "--private-key", "aleo1xx", "--cdn", ""].iter(),
        )
        .unwrap();
        assert!(config.parse_cdn().is_none());

        // Client (Prod)
        let config = Start::try_parse_from(["snarkos", "--client", "--private-key", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config =
            Start::try_parse_from(["snarkos", "--client", "--private-key", "aleo1xx", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config =
            Start::try_parse_from(["snarkos", "--client", "--private-key", "aleo1xx", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());

        // Client (Dev)
        let config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--client", "--private-key", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(
            ["snarkos", "--dev", "0", "--client", "--private-key", "aleo1xx", "--cdn", "url"].iter(),
        )
        .unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(
            ["snarkos", "--dev", "0", "--client", "--private-key", "aleo1xx", "--cdn", ""].iter(),
        )
        .unwrap();
        assert!(config.parse_cdn().is_none());

        // Default (Prod)
        let config = Start::try_parse_from(["snarkos"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(["snarkos", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(["snarkos", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());

        // Default (Dev)
        let config = Start::try_parse_from(["snarkos", "--dev", "0"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(["snarkos", "--dev", "0", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(["snarkos", "--dev", "0", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
    }

    #[test]
    fn test_parse_development() {
        let prod_genesis = Block::from_bytes_le(CurrentNetwork::genesis_bytes()).unwrap();

        let mut trusted_peers = vec![];
        let mut config = Start::try_parse_from(["snarkos"].iter()).unwrap();
        let candidate_genesis = config.parse_development::<CurrentNetwork>(&mut trusted_peers).unwrap();
        assert_eq!(trusted_peers.len(), 0);
        assert_eq!(candidate_genesis, prod_genesis);

        let _config = Start::try_parse_from(["snarkos", "--dev", ""].iter()).unwrap_err();

        // Remove this for Phase 3.
        let mut config =
            Start::try_parse_from(["snarkos", "--dev", "1", "--beacon", "--private-key", ""].iter()).unwrap();
        config.parse_development::<CurrentNetwork>(&mut vec![]).unwrap_err();

        let mut trusted_peers = vec![];
        let mut config = Start::try_parse_from(["snarkos", "--dev", "0"].iter()).unwrap();
        let expected_genesis = config.parse_development::<CurrentNetwork>(&mut trusted_peers).unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4130").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3030").unwrap());
        assert_eq!(trusted_peers.len(), 0);
        assert!(!config.beacon);
        assert!(!config.validator);
        assert!(!config.prover);
        assert!(!config.client);
        assert_ne!(expected_genesis, prod_genesis);

        let mut trusted_peers = vec![];
        let mut config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--beacon", "--private-key", ""].iter()).unwrap();
        let genesis = config.parse_development::<CurrentNetwork>(&mut trusted_peers).unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4130").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3030").unwrap());
        assert_eq!(trusted_peers.len(), 0);
        assert!(config.beacon);
        assert!(!config.validator);
        assert!(!config.prover);
        assert!(!config.client);
        assert_eq!(genesis, expected_genesis);

        let mut trusted_peers = vec![];
        let mut config =
            Start::try_parse_from(["snarkos", "--dev", "1", "--validator", "--private-key", ""].iter()).unwrap();
        let genesis = config.parse_development::<CurrentNetwork>(&mut trusted_peers).unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4131").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3031").unwrap());
        assert_eq!(trusted_peers.len(), 1);
        assert!(!config.beacon);
        assert!(config.validator);
        assert!(!config.prover);
        assert!(!config.client);
        assert_eq!(genesis, expected_genesis);

        let mut trusted_peers = vec![];
        let mut config =
            Start::try_parse_from(["snarkos", "--dev", "2", "--prover", "--private-key", ""].iter()).unwrap();
        let genesis = config.parse_development::<CurrentNetwork>(&mut trusted_peers).unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4132").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3032").unwrap());
        assert_eq!(trusted_peers.len(), 2);
        assert!(!config.beacon);
        assert!(!config.validator);
        assert!(config.prover);
        assert!(!config.client);
        assert_eq!(genesis, expected_genesis);

        let mut trusted_peers = vec![];
        let mut config =
            Start::try_parse_from(["snarkos", "--dev", "3", "--client", "--private-key", ""].iter()).unwrap();
        let genesis = config.parse_development::<CurrentNetwork>(&mut trusted_peers).unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4133").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3033").unwrap());
        assert_eq!(trusted_peers.len(), 3);
        assert!(!config.beacon);
        assert!(!config.validator);
        assert!(!config.prover);
        assert!(config.client);
        assert_eq!(genesis, expected_genesis);
    }

    #[test]
    fn clap_snarkos_start() {
        let arg_vec = vec![
            "snarkos",
            "start",
            "--nodisplay",
            "--dev",
            "2",
            "--validator",
            "--private-key",
            "PRIVATE_KEY",
            "--cdn",
            "CDN",
            "--connect",
            "IP1,IP2,IP3",
            "--rest",
            "127.0.0.1:3033",
        ];
        let cli = CLI::parse_from(arg_vec);

        if let Command::Start(start) = cli.command {
            assert!(start.nodisplay);
            assert_eq!(start.dev, Some(2));
            assert!(start.validator);
            assert_eq!(start.private_key.as_deref(), Some("PRIVATE_KEY"));
            assert_eq!(start.cdn, "CDN");
            assert_eq!(start.rest, "127.0.0.1:3033".parse().unwrap());
            assert_eq!(start.network, 3);
            assert_eq!(start.connect, "IP1,IP2,IP3");
        } else {
            panic!("Unexpected result of clap parsing!");
        }
    }
}
