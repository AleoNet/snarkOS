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

use snarkos_account::Account;
use snarkos_display::Display;
use snarkos_node::{Node, NodeType};
use snarkvm::prelude::{Block, ConsensusMemory, ConsensusStore, FromBytes, Network, PrivateKey, Testnet3, VM};

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
    /// Specify the network of this node.
    #[clap(default_value = "3", long = "network")]
    pub network: u16,
    /// Enables development mode, specify a unique ID for this node.
    #[clap(long)]
    pub dev: Option<u16>,

    /// Specify this as a beacon, with the given account private key for this node.
    #[clap(long = "beacon")]
    pub beacon: Option<String>,
    /// Specify this as a validator, with the given account private key for this node.
    #[clap(long = "validator")]
    pub validator: Option<String>,
    /// Specify this as a prover, with the given account private key for this node.
    #[clap(long = "prover")]
    pub prover: Option<String>,
    /// Specify this as a client, with an optional account private key for this node.
    #[clap(long = "client")]
    pub client: Option<String>,

    /// Specify the IP address and port of a peer to connect to.
    #[clap(default_value = "", long = "connect")]
    pub connect: String,
    /// Specify the IP address and port for the node server.
    #[clap(default_value = "0.0.0.0:4133", long = "node")]
    pub node: SocketAddr,
    /// Specify the IP address and port for the REST server.
    #[clap(default_value = "0.0.0.0:3033", long = "rest")]
    pub rest: SocketAddr,
    /// If the flag is set, the node will not initialize the REST server.
    #[clap(long)]
    pub norest: bool,

    /// Specify the verbosity of the node [options: 0, 1, 2, 3, 4]
    #[clap(default_value = "2", long = "verbosity")]
    pub verbosity: u8,
    /// If the flag is set, the node will not render the display.
    #[clap(long)]
    pub nodisplay: bool,
    /// Enables the node to prefetch initial blocks from a CDN.
    #[clap(default_value = "https://testnet3.blocks.aleo.org/phase2", long = "cdn")]
    pub cdn: String,
    /// Specify the path to the file where logs will be stored.
    #[clap(default_value_os_t = std::env::temp_dir().join("snarkos.log"), long = "logfile")]
    pub logfile: PathBuf,
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
        if self.dev.is_some() || self.cdn.is_empty() || self.client.is_some() || self.prover.is_some() {
            None
        }
        // Check for an edge case, where the node defaults to a client.
        else if let (None, None, None, None) = (&self.beacon, &self.validator, &self.prover, &self.client) {
            None
        }
        // Enable the CDN otherwise.
        else {
            Some(self.cdn.clone())
        }
    }

    /// Updates the configurations if the node is in development mode, and returns the
    /// alternative genesis block if the node is in development mode. Otherwise, returns the actual genesis block.
    fn parse_development<N: Network>(&mut self, trusted_peers: &mut Vec<SocketAddr>) -> Result<Block<N>> {
        // If `--dev` is set, assume the dev nodes are initialized from 0 to `dev`,
        // and add each of them to the trusted peers. In addition, set the node IP to `4130 + dev`,
        // and the REST IP to `3030 + dev`.
        if let Some(dev) = self.dev {
            // Until Phase 3, we only support a single beacon node. To avoid ambiguity, we require
            // the beacon to be the first node in the dev network.
            if dev > 0 && self.beacon.is_some() {
                bail!("Until Phase 3, at most one beacon at '--dev 0' is supported in development mode");
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
            let genesis = Block::genesis(&vm, &beacon_private_key, &mut rng)?;

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
            if self.beacon.is_some() {
                sample_account(&mut self.beacon, true)?;
            }
            // If the node type flag is set, but no private key is provided, then sample one.
            else if let Some("") = self.validator.as_deref() {
                sample_account(&mut self.validator, false)?;
            } else if let Some("") = self.prover.as_deref() {
                sample_account(&mut self.prover, false)?;
            } else if let Some("") = self.client.as_deref() {
                sample_account(&mut self.client, false)?;
            }

            Ok(genesis)
        } else {
            Block::from_bytes_le(N::genesis_bytes())
        }
    }

    /// Returns the node account and node type, from the given configurations.
    fn parse_account<N: Network>(&self) -> Result<(Account<N>, NodeType)> {
        // Ensures only one of the four flags is set. If no flags are set, defaults to a client node.
        match (&self.beacon, &self.validator, &self.prover, &self.client) {
            (Some(private_key), None, None, None) => Ok((Account::<N>::from_str(private_key)?, NodeType::Beacon)),
            (None, Some(private_key), None, None) => Ok((Account::<N>::from_str(private_key)?, NodeType::Validator)),
            (None, None, Some(private_key), None) => Ok((Account::<N>::from_str(private_key)?, NodeType::Prover)),
            (None, None, None, Some(private_key)) => Ok((Account::<N>::from_str(private_key)?, NodeType::Client)),
            (None, None, None, None) => Ok((Account::<N>::new(&mut rand::thread_rng())?, NodeType::Client)),
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

        // Parse the node account and node type.
        let (account, node_type) = self.parse_account::<N>()?;

        // If the display is not enabled, render the welcome message.
        if self.nodisplay {
            // Print the Aleo address.
            println!("🪪 Your Aleo address is {}.\n", account.address().to_string().bold());
            // Print the node type and network.
            println!(
                "🧭 Starting {} on {} at {}.\n",
                node_type.description().bold(),
                N::NAME.bold(),
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
        let config = Start::try_parse_from(["snarkos", "--beacon", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_some());
        let config = Start::try_parse_from(["snarkos", "--beacon", "aleo1xx", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_some());
        let config = Start::try_parse_from(["snarkos", "--beacon", "aleo1xx", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());

        // Beacon (Dev)
        let config = Start::try_parse_from(["snarkos", "--dev", "0", "--beacon", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--beacon", "aleo1xx", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--beacon", "aleo1xx", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());

        // Validator (Prod)
        let config = Start::try_parse_from(["snarkos", "--validator", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_some());
        let config = Start::try_parse_from(["snarkos", "--validator", "aleo1xx", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_some());
        let config = Start::try_parse_from(["snarkos", "--validator", "aleo1xx", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());

        // Validator (Dev)
        let config = Start::try_parse_from(["snarkos", "--dev", "0", "--validator", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--validator", "aleo1xx", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--validator", "aleo1xx", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());

        // Prover (Prod)
        let config = Start::try_parse_from(["snarkos", "--prover", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(["snarkos", "--prover", "aleo1xx", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(["snarkos", "--prover", "aleo1xx", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());

        // Prover (Dev)
        let config = Start::try_parse_from(["snarkos", "--dev", "0", "--prover", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--prover", "aleo1xx", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--prover", "aleo1xx", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());

        // Client (Prod)
        let config = Start::try_parse_from(["snarkos", "--client", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(["snarkos", "--client", "aleo1xx", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config = Start::try_parse_from(["snarkos", "--client", "aleo1xx", "--cdn", ""].iter()).unwrap();
        assert!(config.parse_cdn().is_none());

        // Client (Dev)
        let config = Start::try_parse_from(["snarkos", "--dev", "0", "--client", "aleo1xx"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--client", "aleo1xx", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_none());
        let config =
            Start::try_parse_from(["snarkos", "--dev", "0", "--client", "aleo1xx", "--cdn", ""].iter()).unwrap();
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
        let mut config = Start::try_parse_from(["snarkos", "--dev", "1", "--beacon", ""].iter()).unwrap();
        config.parse_development::<CurrentNetwork>(&mut vec![]).unwrap_err();

        let mut trusted_peers = vec![];
        let mut config = Start::try_parse_from(["snarkos", "--dev", "0"].iter()).unwrap();
        let expected_genesis = config.parse_development::<CurrentNetwork>(&mut trusted_peers).unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4130").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3030").unwrap());
        assert_eq!(trusted_peers.len(), 0);
        assert!(config.beacon.is_none());
        assert!(config.validator.is_none());
        assert!(config.prover.is_none());
        assert!(config.client.is_none());
        assert_ne!(expected_genesis, prod_genesis);

        let mut trusted_peers = vec![];
        let mut config = Start::try_parse_from(["snarkos", "--dev", "0", "--beacon", ""].iter()).unwrap();
        let genesis = config.parse_development::<CurrentNetwork>(&mut trusted_peers).unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4130").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3030").unwrap());
        assert_eq!(trusted_peers.len(), 0);
        assert!(config.beacon.is_some());
        assert!(config.validator.is_none());
        assert!(config.prover.is_none());
        assert!(config.client.is_none());
        assert_eq!(genesis, expected_genesis);

        let mut trusted_peers = vec![];
        let mut config = Start::try_parse_from(["snarkos", "--dev", "1", "--validator", ""].iter()).unwrap();
        let genesis = config.parse_development::<CurrentNetwork>(&mut trusted_peers).unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4131").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3031").unwrap());
        assert_eq!(trusted_peers.len(), 1);
        assert!(config.beacon.is_none());
        assert!(config.validator.is_some());
        assert!(config.prover.is_none());
        assert!(config.client.is_none());
        assert_eq!(genesis, expected_genesis);

        let mut trusted_peers = vec![];
        let mut config = Start::try_parse_from(["snarkos", "--dev", "2", "--prover", ""].iter()).unwrap();
        let genesis = config.parse_development::<CurrentNetwork>(&mut trusted_peers).unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4132").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3032").unwrap());
        assert_eq!(trusted_peers.len(), 2);
        assert!(config.beacon.is_none());
        assert!(config.validator.is_none());
        assert!(config.prover.is_some());
        assert!(config.client.is_none());
        assert_eq!(genesis, expected_genesis);

        let mut trusted_peers = vec![];
        let mut config = Start::try_parse_from(["snarkos", "--dev", "3", "--client", ""].iter()).unwrap();
        let genesis = config.parse_development::<CurrentNetwork>(&mut trusted_peers).unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4133").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3033").unwrap());
        assert_eq!(trusted_peers.len(), 3);
        assert!(config.beacon.is_none());
        assert!(config.validator.is_none());
        assert!(config.prover.is_none());
        assert!(config.client.is_some());
        assert_eq!(genesis, expected_genesis);
    }
}
