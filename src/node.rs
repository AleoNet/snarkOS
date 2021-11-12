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

use crate::{network::Server, Client, ClientTrial, Display, Environment, Miner, MinerTrial, NodeType};
use snarkvm::{
    dpc::{prelude::*, testnet2::Testnet2},
    prelude::*,
};

use anyhow::Result;
use std::str::FromStr;
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
}

impl Node {
    /// Starts the node.
    pub async fn start(self) -> Result<()> {
        match (self.network, self.miner.is_some(), self.trial) {
            (2, true, false) => self.start_server::<Testnet2, Miner<Testnet2>>().await,
            (2, false, false) => self.start_server::<Testnet2, Client<Testnet2>>().await,
            (2, true, true) => self.start_server::<Testnet2, MinerTrial<Testnet2>>().await,
            (2, false, true) => self.start_server::<Testnet2, ClientTrial<Testnet2>>().await,
            _ => panic!("Unsupported node configuration"),
        }
    }

    async fn start_server<N: Network, E: Environment>(&self) -> Result<()> {
        let node_port = self.node.unwrap_or(E::DEFAULT_NODE_PORT);
        let rpc_port = self.rpc.unwrap_or(E::DEFAULT_RPC_PORT);
        if node_port < 4130 {
            panic!("Until configuration files are established, the node port must be at least 4130 or greater");
        }

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
