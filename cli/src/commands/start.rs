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
use snarkos_node::{narwhal::MEMORY_POOL_PORT, router::messages::NodeType, Node};
use snarkvm::{
    console::{
        account::{Address, PrivateKey},
        algorithms::Hash,
        network::{Network, Testnet3},
    },
    ledger::{
        block::Block,
        committee::{Committee, MIN_VALIDATOR_STAKE},
        store::{helpers::memory::ConsensusMemory, ConsensusStore},
    },
    prelude::{FromBytes, ToBits, ToBytes},
    synthesizer::VM,
    utilities::to_bytes_le,
};

use anyhow::{bail, Result};
use clap::Parser;
use colored::Colorize;
use core::str::FromStr;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use std::{net::SocketAddr, path::PathBuf};
use tokio::runtime::{self, Runtime};

/// The recommended minimum number of 'open files' limit for a validator.
/// Validators should be able to handle at least 1000 concurrent connections, each requiring 2 sockets.
#[cfg(target_family = "unix")]
const RECOMMENDED_MIN_NOFILES_LIMIT: u64 = 2048;

/// The development mode RNG seed.
const DEVELOPMENT_MODE_RNG_SEED: u64 = 1234567890u64;
/// The development mode number of genesis committee members.
const DEVELOPMENT_MODE_NUM_GENESIS_COMMITTEE_MEMBERS: u16 = 4;
/// The development mode number of nodes with public balance.
const DEVELOPMENT_MODE_NUM_NODES_WITH_PUBLIC_BALANCE: u16 = 50;

/// Starts the snarkOS node.
#[derive(Clone, Debug, Parser)]
pub struct Start {
    /// Specify the network ID of this node
    #[clap(default_value = "3", long = "network")]
    pub network: u16,

    /// Specify this node as a validator
    #[clap(long = "validator")]
    pub validator: bool,
    /// Specify this node as a prover
    #[clap(long = "prover")]
    pub prover: bool,
    /// Specify this node as a client
    #[clap(long = "client")]
    pub client: bool,

    /// Specify the account private key of the node
    #[clap(long = "private-key")]
    pub private_key: Option<String>,
    /// Specify the path to a file containing the account private key of the node
    #[clap(long = "private-key-file")]
    pub private_key_file: Option<PathBuf>,

    /// Specify the IP address and port for the node server
    #[clap(default_value = "0.0.0.0:4133", long = "node")]
    pub node: SocketAddr,
    /// Specify the IP address and port of the peer(s) to connect to
    #[clap(default_value = "", long = "peers")]
    pub peers: String,
    /// Specify the IP address and port of the validator(s) to connect to
    #[clap(default_value = "", long = "validators")]
    pub validators: String,

    // Specify the IP address and port for narwhal.
    #[clap(long = "narwhal")]
    pub narwhal: Option<SocketAddr>,

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
    #[clap(default_value = "1", long = "verbosity")]
    pub verbosity: u8,
    /// Specify the path to the file where logs will be stored
    #[clap(default_value_os_t = std::env::temp_dir().join("snarkos.log"), long = "logfile")]
    pub logfile: PathBuf,

    /// Enables the node to prefetch initial blocks from a CDN
    #[clap(default_value = "https://s3.us-west-1.amazonaws.com/testnet3.blocks/phase3", long = "cdn")]
    pub cdn: String,
    /// Enables development mode, specify a unique ID for this node
    #[clap(long)]
    pub dev: Option<u16>,
    /// If development mode is enabled, specify the number of genesis validators (default: 4)
    #[clap(long)]
    pub dev_num_validators: Option<u16>,
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
    /// Returns the initial peer(s) to connect to, from the given configurations.
    fn parse_trusted_peers(&self) -> Result<Vec<SocketAddr>> {
        match self.peers.is_empty() {
            true => Ok(vec![]),
            false => Ok(self
                .peers
                .split(',')
                .flat_map(|ip| match ip.parse::<SocketAddr>() {
                    Ok(ip) => Some(ip),
                    Err(e) => {
                        eprintln!("The IP supplied to --peers ('{ip}') is malformed: {e}");
                        None
                    }
                })
                .collect()),
        }
    }

    /// Returns the initial validator(s) to connect to, from the given configurations.
    fn parse_trusted_validators(&self) -> Result<Vec<SocketAddr>> {
        match self.validators.is_empty() {
            true => Ok(vec![]),
            false => Ok(self
                .validators
                .split(',')
                .flat_map(|ip| match ip.parse::<SocketAddr>() {
                    Ok(ip) => Some(ip),
                    Err(e) => {
                        eprintln!("The IP supplied to --validators ('{ip}') is malformed: {e}");
                        None
                    }
                })
                .collect()),
        }
    }

    /// Returns the CDN to prefetch initial blocks from, from the given configurations.
    fn parse_cdn(&self) -> Option<String> {
        // Determine if the node type is not declared.
        let is_no_node_type = !(self.validator || self.prover || self.client);

        // Disable CDN if:
        //  1. The node is in development mode.
        //  2. The user has explicitly disabled CDN.
        //  3. The node is a prover (no need to sync).
        //  4. The node type is not declared (defaults to client) (no need to sync).
        if self.dev.is_some() || self.cdn.is_empty() || self.prover || is_no_node_type {
            None
        }
        // Enable the CDN otherwise.
        else {
            Some(self.cdn.clone())
        }
    }

    /// Read the private key directly from an argument or from a filesystem location,
    /// returning the Aleo account.
    fn parse_private_key<N: Network>(&self) -> Result<Account<N>> {
        match (&self.private_key, &self.private_key_file) {
            // Parse the private key directly.
            (Some(private_key), None) => Account::from_str(private_key.trim()),
            // Parse the private key from a file.
            (None, Some(path)) => Account::from_str(std::fs::read_to_string(path)?.trim()),
            // Ensure the private key is provided to the CLI, except for clients or nodes in development mode.
            (None, None) => {
                if self.client {
                    Account::new(&mut rand::thread_rng())
                } else if let Some(dev) = self.dev {
                    // Sample the private key of this node.
                    Account::try_from({
                        // Initialize the (fixed) RNG.
                        let mut rng = ChaChaRng::seed_from_u64(DEVELOPMENT_MODE_RNG_SEED);
                        // Iterate through 'dev' address instances to match the account.
                        for _ in 0..dev {
                            let _ = PrivateKey::<N>::new(&mut rng)?;
                        }
                        PrivateKey::<N>::new(&mut rng)?
                    })
                } else {
                    bail!("Missing the '--private-key' or '--private-key-file' argument")
                }
            }
            // Ensure only one private key flag is provided to the CLI.
            (Some(_), Some(_)) => {
                bail!("Cannot use '--private-key' and '--private-key-file' simultaneously, please use only one")
            }
        }
    }

    /// Updates the configurations if the node is in development mode.
    fn parse_development(
        &mut self,
        trusted_peers: &mut Vec<SocketAddr>,
        trusted_validators: &mut Vec<SocketAddr>,
    ) -> Result<()> {
        // If `--dev` is set, assume the dev nodes are initialized from 0 to `dev`,
        // and add each of them to the trusted peers. In addition, set the node IP to `4130 + dev`,
        // and the REST IP to `3030 + dev`.
        if let Some(dev) = self.dev {
            // Add the dev nodes to the trusted peers.
            if trusted_peers.is_empty() {
                for i in 0..dev {
                    if i != dev {
                        trusted_peers.push(SocketAddr::from_str(&format!("127.0.0.1:{}", 4130 + i))?);
                    }
                }
            }
            if trusted_validators.is_empty() {
                // To avoid ambiguity, we define the first few nodes to be the trusted validators to connect to.
                for i in 0..2 {
                    if i != dev {
                        trusted_validators.push(SocketAddr::from_str(&format!("127.0.0.1:{}", MEMORY_POOL_PORT + i))?);
                    }
                }
            }
            // Set the node IP to `4130 + dev`.
            self.node = SocketAddr::from_str(&format!("0.0.0.0:{}", 4130 + dev))?;
            // Set the REST IP to `3030 + dev`.
            if !self.norest {
                self.rest = SocketAddr::from_str(&format!("0.0.0.0:{}", 3030 + dev))?;
            }
        }
        Ok(())
    }

    /// Returns an alternative genesis block if the node is in development mode.
    /// Otherwise, returns the actual genesis block.
    fn parse_genesis<N: Network>(&self) -> Result<Block<N>> {
        if self.dev.is_some() {
            // Determine the number of genesis committee members.
            let num_genesis_committee_members = match self.dev_num_validators {
                Some(num_genesis_committee_members) => num_genesis_committee_members,
                None => DEVELOPMENT_MODE_NUM_GENESIS_COMMITTEE_MEMBERS,
            };

            // Initialize the (fixed) RNG.
            let mut rng = ChaChaRng::seed_from_u64(DEVELOPMENT_MODE_RNG_SEED);
            // Initialize the development private keys.
            let development_private_keys = (0..DEVELOPMENT_MODE_NUM_NODES_WITH_PUBLIC_BALANCE)
                .map(|_| PrivateKey::<N>::new(&mut rng))
                .collect::<Result<Vec<_>>>()?;

            // Construct the committee members.
            let members = development_private_keys
                .iter()
                .take(num_genesis_committee_members as usize)
                .map(|private_key| Ok((Address::try_from(private_key)?, (MIN_VALIDATOR_STAKE, true))))
                .collect::<Result<indexmap::IndexMap<_, _>>>()?;
            // Construct the committee.
            let committee = Committee::<N>::new(0u64, members)?;

            // Determine the public balance per validator.
            let public_balance_per_validator = (N::STARTING_SUPPLY
                - (num_genesis_committee_members as u64 * MIN_VALIDATOR_STAKE))
                / (DEVELOPMENT_MODE_NUM_NODES_WITH_PUBLIC_BALANCE as u64);
            assert_eq!(
                N::STARTING_SUPPLY,
                (num_genesis_committee_members as u64 * MIN_VALIDATOR_STAKE)
                    + (DEVELOPMENT_MODE_NUM_NODES_WITH_PUBLIC_BALANCE as u64 * public_balance_per_validator),
                "The public balance per validator is not correct."
            );

            // Construct the public balances.
            let public_balances = development_private_keys
                .iter()
                .map(|private_key| Ok((Address::try_from(private_key)?, public_balance_per_validator)))
                .collect::<Result<indexmap::IndexMap<_, _>>>()?;

            // Construct the genesis block.
            load_or_compute_genesis(development_private_keys[0], committee, public_balances, &mut rng)
        } else {
            // If the `dev_num_validators` flag is set, inform the user that it is ignored.
            if self.dev_num_validators.is_some() {
                eprintln!("The '--dev-num-validators' flag is ignored because '--dev' is not set");
            }

            Block::from_bytes_le(N::genesis_bytes())
        }
    }

    /// Returns the node type, from the given configurations.
    const fn parse_node_type(&self) -> NodeType {
        if self.validator {
            NodeType::Validator
        } else if self.prover {
            NodeType::Prover
        } else {
            NodeType::Client
        }
    }

    /// Returns the node type corresponding to the given configurations.
    #[rustfmt::skip]
    async fn parse_node<N: Network>(&mut self) -> Result<Node<N>> {
        // Print the welcome.
        println!("{}", crate::helpers::welcome_message());

        // Parse the trusted peers to connect to.
        let mut trusted_peers = self.parse_trusted_peers()?;
        // Parse the trusted validators to connect to.
        let mut trusted_validators = self.parse_trusted_validators()?;
        // Parse the development configurations.
        self.parse_development(&mut trusted_peers, &mut trusted_validators)?;

        // Parse the CDN.
        let cdn = self.parse_cdn();

        // Parse the genesis block.
        let genesis = self.parse_genesis::<N>()?;
        // Parse the private key of the node.
        let account = self.parse_private_key::<N>()?;
        // Parse the node type.
        let node_type = self.parse_node_type();

        // Parse the REST IP.
        let rest_ip = match self.norest {
            true => None,
            false => Some(self.rest),
        };

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
            if node_type.is_validator() {
                if let Some(rest_ip) = rest_ip {
                    println!("🌐 Starting the REST server at {}.\n", rest_ip.to_string().bold());

                    if let Ok(jwt_token) = snarkos_node_rest::Claims::new(account.address()).to_jwt_string() {
                        println!("🔑 Your one-time JWT token is {}\n", jwt_token.dimmed());
                    }
                }
            }
        }

        // If the node is a validator, check if the open files limit is lower than recommended.
        #[cfg(target_family = "unix")]
        if node_type.is_validator() {
            crate::helpers::check_open_files_limit(RECOMMENDED_MIN_NOFILES_LIMIT);
        }
        // Check if the machine meets the minimum requirements for a validator.
        crate::helpers::check_validator_machine(node_type);

        // Initialize the node.
        let narwhal_ip = if self.dev.is_some() { self.narwhal } else { None };
        match node_type {
            NodeType::Validator => Node::new_validator(self.node, rest_ip, narwhal_ip, account, &trusted_peers, &trusted_validators, genesis, cdn, self.dev).await,
            NodeType::Prover => Node::new_prover(self.node, account, &trusted_peers, genesis, self.dev).await,
            NodeType::Client => Node::new_client(self.node, rest_ip, account, &trusted_peers, genesis, cdn, self.dev).await,
        }
    }

    /// Returns a runtime for the node.
    fn runtime() -> Runtime {
        // Retrieve the number of cores.
        let num_cores = num_cpus::get();
        // Determine the number of main cores.
        let main_cores = match num_cores {
            // Insufficient
            0..=3 => unreachable!("The number of cores is insufficient"),
            // Efficiency mode
            4..=8 => 2,
            // Standard mode
            9..=16 => 4,
            // Performance mode
            _ => 8,
        };

        let (num_tokio_worker_threads, max_tokio_blocking_threads, num_rayon_cores_global) =
            { (num_cores.min(main_cores), 512, num_cores.saturating_sub(main_cores).max(1)) };

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

/// Loads or computes the genesis block.
fn load_or_compute_genesis<N: Network>(
    genesis_private_key: PrivateKey<N>,
    committee: Committee<N>,
    public_balances: indexmap::IndexMap<Address<N>, u64>,
    rng: &mut ChaChaRng,
) -> Result<Block<N>> {
    // Construct the preimage.
    let bytes = to_bytes_le![genesis_private_key, committee, public_balances.iter().collect::<Vec<(_, _)>>()]?;
    // Initialize the hasher.
    let hasher = snarkvm::console::algorithms::BHP256::<N>::setup("aleo.dev.block")?;
    // Compute the hash.
    let hash = hasher.hash(&bytes.to_bits_le())?.to_string();

    // A closure to load the block.
    let load_block = |file_path| -> Result<Block<N>> {
        // Attempts to load the genesis block file locally.
        let buffer = std::fs::read(file_path)?;
        // Return the genesis block.
        Block::from_bytes_le(&buffer)
    };

    // Construct the file path.
    let file_path = std::env::temp_dir().join(hash);
    // Check if the genesis block exists.
    if file_path.exists() {
        // If the block loads successfully, return it.
        if let Ok(block) = load_block(&file_path) {
            return Ok(block);
        }
    }

    /* Otherwise, compute the genesis block and store it. */

    // Initialize a new VM.
    let vm = VM::from(ConsensusStore::<N, ConsensusMemory<N>>::open(None)?)?;
    // Initialize the genesis block.
    let block = vm.genesis_quorum(&genesis_private_key, committee, public_balances, rng)?;
    // Write the genesis block to the file.
    std::fs::write(&file_path, block.to_bytes_le()?)?;
    // Return the genesis block.
    Ok(block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Command, CLI};
    use snarkvm::prelude::Testnet3;

    type CurrentNetwork = Testnet3;

    #[test]
    fn test_parse_trusted_peers() {
        let config = Start::try_parse_from(["snarkos", "--peers", ""].iter()).unwrap();
        assert!(config.parse_trusted_peers().is_ok());
        assert!(config.parse_trusted_peers().unwrap().is_empty());

        let config = Start::try_parse_from(["snarkos", "--peers", "1.2.3.4:5"].iter()).unwrap();
        assert!(config.parse_trusted_peers().is_ok());
        assert_eq!(config.parse_trusted_peers().unwrap(), vec![SocketAddr::from_str("1.2.3.4:5").unwrap()]);

        let config = Start::try_parse_from(["snarkos", "--peers", "1.2.3.4:5,6.7.8.9:0"].iter()).unwrap();
        assert!(config.parse_trusted_peers().is_ok());
        assert_eq!(config.parse_trusted_peers().unwrap(), vec![
            SocketAddr::from_str("1.2.3.4:5").unwrap(),
            SocketAddr::from_str("6.7.8.9:0").unwrap()
        ]);
    }

    #[test]
    fn test_parse_trusted_validators() {
        let config = Start::try_parse_from(["snarkos", "--validators", ""].iter()).unwrap();
        assert!(config.parse_trusted_validators().is_ok());
        assert!(config.parse_trusted_validators().unwrap().is_empty());

        let config = Start::try_parse_from(["snarkos", "--validators", "1.2.3.4:5"].iter()).unwrap();
        assert!(config.parse_trusted_validators().is_ok());
        assert_eq!(config.parse_trusted_validators().unwrap(), vec![SocketAddr::from_str("1.2.3.4:5").unwrap()]);

        let config = Start::try_parse_from(["snarkos", "--validators", "1.2.3.4:5,6.7.8.9:0"].iter()).unwrap();
        assert!(config.parse_trusted_validators().is_ok());
        assert_eq!(config.parse_trusted_validators().unwrap(), vec![
            SocketAddr::from_str("1.2.3.4:5").unwrap(),
            SocketAddr::from_str("6.7.8.9:0").unwrap()
        ]);
    }

    #[test]
    fn test_parse_cdn() {
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
        assert!(config.parse_cdn().is_some());
        let config =
            Start::try_parse_from(["snarkos", "--client", "--private-key", "aleo1xx", "--cdn", "url"].iter()).unwrap();
        assert!(config.parse_cdn().is_some());
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
    fn test_parse_development_and_genesis() {
        let prod_genesis = Block::from_bytes_le(CurrentNetwork::genesis_bytes()).unwrap();

        let mut trusted_peers = vec![];
        let mut trusted_validators = vec![];
        let mut config = Start::try_parse_from(["snarkos"].iter()).unwrap();
        config.parse_development(&mut trusted_peers, &mut trusted_validators).unwrap();
        let candidate_genesis = config.parse_genesis::<CurrentNetwork>().unwrap();
        assert_eq!(trusted_peers.len(), 0);
        assert_eq!(trusted_validators.len(), 0);
        assert_eq!(candidate_genesis, prod_genesis);

        let _config = Start::try_parse_from(["snarkos", "--dev", ""].iter()).unwrap_err();

        let mut trusted_peers = vec![];
        let mut trusted_validators = vec![];
        let mut config = Start::try_parse_from(["snarkos", "--dev", "0"].iter()).unwrap();
        config.parse_development(&mut trusted_peers, &mut trusted_validators).unwrap();
        let expected_genesis = config.parse_genesis::<CurrentNetwork>().unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4130").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3030").unwrap());
        assert_eq!(trusted_peers.len(), 0);
        assert_eq!(trusted_validators.len(), 1);
        assert!(!config.validator);
        assert!(!config.prover);
        assert!(!config.client);
        assert_ne!(expected_genesis, prod_genesis);

        let mut trusted_peers = vec![];
        let mut trusted_validators = vec![];
        let mut config =
            Start::try_parse_from(["snarkos", "--dev", "1", "--validator", "--private-key", ""].iter()).unwrap();
        config.parse_development(&mut trusted_peers, &mut trusted_validators).unwrap();
        let genesis = config.parse_genesis::<CurrentNetwork>().unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4131").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3031").unwrap());
        assert_eq!(trusted_peers.len(), 1);
        assert_eq!(trusted_validators.len(), 1);
        assert!(config.validator);
        assert!(!config.prover);
        assert!(!config.client);
        assert_eq!(genesis, expected_genesis);

        let mut trusted_peers = vec![];
        let mut trusted_validators = vec![];
        let mut config =
            Start::try_parse_from(["snarkos", "--dev", "2", "--prover", "--private-key", ""].iter()).unwrap();
        config.parse_development(&mut trusted_peers, &mut trusted_validators).unwrap();
        let genesis = config.parse_genesis::<CurrentNetwork>().unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4132").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3032").unwrap());
        assert_eq!(trusted_peers.len(), 2);
        assert_eq!(trusted_validators.len(), 2);
        assert!(!config.validator);
        assert!(config.prover);
        assert!(!config.client);
        assert_eq!(genesis, expected_genesis);

        let mut trusted_peers = vec![];
        let mut trusted_validators = vec![];
        let mut config =
            Start::try_parse_from(["snarkos", "--dev", "3", "--client", "--private-key", ""].iter()).unwrap();
        config.parse_development(&mut trusted_peers, &mut trusted_validators).unwrap();
        let genesis = config.parse_genesis::<CurrentNetwork>().unwrap();
        assert_eq!(config.node, SocketAddr::from_str("0.0.0.0:4133").unwrap());
        assert_eq!(config.rest, SocketAddr::from_str("0.0.0.0:3033").unwrap());
        assert_eq!(trusted_peers.len(), 3);
        assert_eq!(trusted_validators.len(), 2);
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
            "--peers",
            "IP1,IP2,IP3",
            "--validators",
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
            assert_eq!(start.peers, "IP1,IP2,IP3");
            assert_eq!(start.validators, "IP1,IP2,IP3");
        } else {
            panic!("Unexpected result of clap parsing!");
        }
    }
}
