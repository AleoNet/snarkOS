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
use snarkvm::prelude::PrivateKey;

use anyhow::{bail, Result};
use clap::Parser;
use core::str::FromStr;
use std::net::SocketAddr;
use tokio::runtime::{self, Runtime};

type Network = snarkvm::prelude::Testnet3;

/// Starts the snarkOS node.
#[derive(Debug, Clone, Parser)]
pub struct Start {
    /// Specify the network of this node.
    #[clap(default_value = "3", long = "network")]
    pub network: u16,

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
    #[clap(parse(try_from_str), default_value = "0.0.0.0:3033")]
    pub rest: SocketAddr,
    /// If the flag is set, the node will not initialize the REST server.
    #[clap(long)]
    pub norest: bool,

    /// Specify the verbosity of the node [options: 0, 1, 2, 3]
    #[clap(default_value = "2", long = "verbosity")]
    pub verbosity: u8,
    /// Enables development mode, specify a unique ID for the local node.
    #[clap(long)]
    pub dev: Option<u16>,
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
            // Parse the node from the configurations.
            let node = cli.parse_node().await.expect("Failed to parse the node");
            // Initialize the display.
            let _ = Display::start(node, cli.verbosity, cli.nodisplay).expect("Failed to initialize the display");
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

    /// Returns the node type corresponding to the given configurations.
    #[rustfmt::skip]
    async fn parse_node(&mut self) -> Result<Node<Network>> {
        // Parse the trusted IPs to connect to.
        let mut trusted_peers = self.parse_trusted_peers()?;
        // Parse the node IP.
        let mut node_ip = self.node;

        // If `--dev` is set, assume the dev nodes are initialized from 0 to `dev`,
        // and add each of them to the trusted peers. In addition, set the node IP to `4130 + dev`.
        if let Some(dev) = self.dev {
            // Add the dev nodes to the trusted peers.
            for i in 0..dev {
                trusted_peers.push(SocketAddr::from_str(&format!("127.0.0.1:{}", 4130 + i))?);
            }
            // Set the node IP to `4130 + dev`.
            node_ip = SocketAddr::from_str(&format!("0.0.0.0:{}", 4130 + dev))?;

            // If the node type flag is set, but no private key is provided, then sample one.
            let sample_account = |node: &mut Option<String>| -> Result<()> {
                let account = Account::<Network>::sample()?;
                *node = Some(account.private_key().to_string());
                println!("ATTENTION - No private key was provided, sampling a one-time account for this instance:\n\n{account}\n");
                Ok(())
            };
            if let Some("") = self.beacon.as_ref().map(|s| s.as_str()) {
                sample_account(&mut self.beacon)?;
            } else if let Some("") = self.validator.as_ref().map(|s| s.as_str()) {
                sample_account(&mut self.validator)?;
            } else if let Some("") = self.prover.as_ref().map(|s| s.as_str()) {
                sample_account(&mut self.prover)?;
            } else if let Some("") = self.client.as_ref().map(|s| s.as_str()) {
                sample_account(&mut self.client)?;
            }
        }

        // Ensures only one of the four flags is set. If no flags are set, defaults to a client node.
        match (&self.beacon, &self.validator, &self.prover, &self.client) {
            (Some(private_key), None, None, None) => Node::new_beacon(node_ip, PrivateKey::<Network>::from_str(private_key)?, &trusted_peers, self.dev).await,
            (None, Some(private_key), None, None) => Node::new_validator(node_ip, PrivateKey::<Network>::from_str(private_key)?, &trusted_peers, self.dev).await,
            (None, None, Some(private_key), None) => Node::new_prover(node_ip, PrivateKey::<Network>::from_str(private_key)?, &trusted_peers, self.dev).await,
            (None, None, None, Some(private_key)) => Node::new_client(node_ip, PrivateKey::<Network>::from_str(private_key)?, &trusted_peers, self.dev).await,
            (None, None, None, None) => Node::new_client(node_ip, PrivateKey::<Network>::new(&mut rand::thread_rng())?, &trusted_peers, self.dev).await,
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
            { ((num_cpus::get() / 8 * 2).max(1), num_cpus::get(), (num_cpus::get() / 8 * 5).max(1)) };

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
