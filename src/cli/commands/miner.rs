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
    cli::{commands::*, helpers::*},
    Miner,
    Node,
};

use ::rand::thread_rng;
use snarkvm::{
    dpc::{prelude::*, testnet2::Testnet2},
    prelude::*,
};
use std::str::FromStr;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(setting = structopt::clap::AppSettings::ColoredHelp)]
pub enum MinerMode {
    /// Starts a new miner node.
    Start {
        /// The node port to receive incoming messages.
        #[structopt(short = "n", long = "node-port")]
        node_port: Option<u16>,

        /// The rpc port to receive incoming requests.
        #[structopt(short = "r", long = "rpc-port")]
        rpc_port: Option<u16>,

        /// The miner's aleo private key
        #[structopt(short = "p", long = "private-key")]
        private_key: Option<String>,
    },
}

impl MinerMode {
    /// Starts the miner node
    pub async fn start(self) -> anyhow::Result<()> {
        match self {
            Self::Start {
                node_port,
                rpc_port,
                private_key,
            } => {
                let node_port = node_port.unwrap_or(DEFAULT_NODE_PORT);
                let rpc_port = rpc_port.unwrap_or(DEFAULT_RPC_PORT);

                // If the user does not provide a private key, then sample one from random.
                let miner_private_key = match private_key {
                    Some(private_key) => PrivateKey::from_str(&private_key)?,
                    None => PrivateKey::new(&mut thread_rng()),
                };

                if node_port < 4130 {
                    panic!("Until configuration files are established, the port must be at least 4130 or greater");
                }

                initialize_logger();
                print_welcome();

                let account = Account::<Testnet2>::from(miner_private_key);

                let _node =
                    Node::<Testnet2, Miner>::new(node_port, rpc_port, (node_port as u16 - 4130) as u8, Some(account.address())).await?;
            }
        }

        Ok(())
    }
}
