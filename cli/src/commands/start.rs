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
use snarkos_node::Node;
use snarkvm::prelude::{Block, ConsensusMemory, ConsensusStore, Network, PrivateKey, Testnet3, VM};

use anyhow::{bail, Result};
use clap::Parser;
use core::str::FromStr;
use rand::{seq::SliceRandom, SeedableRng};
use rand_chacha::ChaChaRng;
use std::net::SocketAddr;
use tokio::runtime::{self, Runtime};

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

    /// Specify the verbosity of the node [options: 0, 1, 2, 3]
    #[clap(default_value = "2", long = "verbosity")]
    pub verbosity: u8,
    /// If the flag is set, the node will not render the display.
    #[clap(long)]
    pub nodisplay: bool,
}

impl Start {
    /// Starts the snarkOS node.
    pub fn parse(self) -> Result<String> {
        // Initialize the runtime.
        Self::runtime().block_on(async move {
            // Clone the configurations.
            let mut cli = self.clone();
            // Parse the network.
            match cli.network {
                3 => {
                    // Parse the node from the configurations.
                    let node = cli.parse_node::<Testnet3>().await.expect("Failed to parse the node");
                    // Initialize the display.
                    Display::start(node, cli.verbosity, cli.nodisplay).expect("Failed to initialize the display");
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

    /// Updates the configurations if the node is in development mode.
    fn parse_development<N: Network>(&mut self, trusted_peers: &mut Vec<SocketAddr>) -> Result<Option<Block<N>>> {
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
                    true => Account::<N>::from(beacon_private_key)?,
                    false => Account::<N>::sample()?,
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

            Ok(Some(genesis))
        } else {
            // Prepare the bootstrap.
            let bootstrap = [
                "164.92.111.59:4133",
                "159.223.204.96:4133",
                "167.71.219.176:4133",
                "157.245.205.209:4133",
                "134.122.95.106:4133",
                "161.35.24.55:4133",
            ];

            // Include a bootstrap node, as the node is not in development mode.
            match bootstrap.choose(&mut rand::thread_rng()) {
                Some(ip) => trusted_peers.push(SocketAddr::from_str(ip)?),
                None => bail!("Failed to choose a bootstrap node"),
            }

            Ok(None)
        }
    }

    /// Returns the node type corresponding to the given configurations.
    #[rustfmt::skip]
    async fn parse_node<N: Network>(&mut self) -> Result<Node<N>> {
        // Print the welcome.
        println!("{}", Display::<N>::welcome_message());

        // Parse the trusted IPs to connect to.
        let mut trusted_peers = self.parse_trusted_peers()?;

        // Parse the development configurations, and determine the genesis block.
        let genesis = self.parse_development::<N>(&mut trusted_peers)?;

        // Parse the REST IP.
        let rest_ip = match self.norest {
            true => None,
            false => Some(self.rest),
        };

        // Ensures only one of the four flags is set. If no flags are set, defaults to a client node.
        match (&self.beacon, &self.validator, &self.prover, &self.client) {
            (Some(private_key), None, None, None) => Node::new_beacon(self.node, rest_ip, PrivateKey::<N>::from_str(private_key)?, &trusted_peers, genesis, self.dev).await,
            (None, Some(private_key), None, None) => Node::new_validator(self.node, rest_ip, PrivateKey::<N>::from_str(private_key)?, &trusted_peers, genesis, self.dev).await,
            (None, None, Some(private_key), None) => Node::new_prover(self.node, PrivateKey::<N>::from_str(private_key)?, &trusted_peers).await,
            (None, None, None, Some(private_key)) => Node::new_client(self.node, PrivateKey::<N>::from_str(private_key)?, &trusted_peers).await,
            (None, None, None, None) => Node::new_client(self.node, PrivateKey::<N>::new(&mut rand::thread_rng())?, &trusted_peers).await,
            _ => bail!("Unsupported node configuration"),
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
            { (num_cpus::get().min(4), 512, num_cpus::get().saturating_sub(4).max(1)) };

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
    fn test_parse_development() {
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
        assert!(expected_genesis.is_some());

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
