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

use crate::cli::display::*;

use snarkvm::{
    dpc::{prelude::*, testnet2::Testnet2},
    prelude::*,
};

use crate::{Client, Miner, Node};
use ::rand::thread_rng;
use std::str::FromStr;
use structopt::StructOpt;

pub const DEFAULT_NODE_PORT: u16 = 4132;
pub const DEFAULT_RPC_PORT: u16 = 3032;

#[derive(StructOpt, Debug)]
#[structopt(name = "snarkos", author = "The Aleo Team <hello@aleo.org>", setting = structopt::clap::AppSettings::ColoredHelp)]
pub struct CLI {
    /* ==================== flags ==================== */
    /// Enable debug mode
    #[structopt(short, long)]
    pub debug: bool,

    /// Start mining blocks from this node
    #[structopt(long = "is-miner")]
    pub is_miner: bool,

    /* ==================== options ==================== */
    /// Specify the address that will receive miner rewards
    #[structopt(long = "miner-address")]
    pub miner_address: Option<String>,

    /// Specify the port the node is run on
    #[structopt(short = "p", long = "port")]
    pub port: Option<u16>,

    /// Specify the port the json rpc server is run on
    #[structopt(long = "rpc-port")]
    pub rpc_port: Option<u16>,

    /// Specify the verbosity (default = 1) of the node [possible values: 0, 1, 2, 3]
    #[structopt(long)]
    pub verbose: Option<u8>,
}

impl CLI {
    /// Starts the node.
    pub async fn start(self) -> anyhow::Result<()> {
        let node_port = self.port.unwrap_or(DEFAULT_NODE_PORT);
        let rpc_port = self.rpc_port.unwrap_or(DEFAULT_RPC_PORT);

        if node_port < 4130 {
            panic!("Until configuration files are established, the port must be at least 4130 or greater");
        }

        initialize_logger(self.verbose);
        print_welcome();

        if self.is_miner {
            let miner_address = match self.miner_address {
                Some(address) => Address::<Testnet2>::from_str(&address)?,
                None => Account::<Testnet2>::new(&mut thread_rng()).address(),
            };
            println!("Your Aleo address is {}.\n\n", miner_address);
            println!("Starting a mining node on testnet2.\n");

            let _node = Node::<Testnet2, Miner>::new(node_port, rpc_port, (node_port as u16 - 4130) as u8, Some(miner_address)).await?;
        } else {
            println!("Starting a client node on testnet2.\n");
            let _node = Node::<Testnet2, Client>::new(node_port, rpc_port, (node_port as u16 - 4130) as u8, None).await?;
        }

        Ok(())
    }
}
