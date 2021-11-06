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
    Client,
    Node,
};

use snarkvm::{
    dpc::{prelude::*, testnet2::Testnet2},
    prelude::*,
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(setting = structopt::clap::AppSettings::ColoredHelp)]
pub enum ClientMode {
    /// Starts a new client node.
    Start {
        /// The node port to receive incoming messages.
        #[structopt(short = "n", long = "node-port")]
        node_port: Option<u16>,

        /// The rpc port to receive incoming requests.
        #[structopt(short = "r", long = "rpc-port")]
        rpc_port: Option<u16>,
    },
}

impl ClientMode {
    /// Starts the client node
    pub async fn start(self) -> anyhow::Result<()> {
        match self {
            Self::Start { node_port, rpc_port } => {
                let node_port = node_port.unwrap_or(DEFAULT_NODE_PORT);
                let rpc_port = rpc_port.unwrap_or(DEFAULT_RPC_PORT);

                if node_port < 4130 {
                    panic!("Until configuration files are established, the port must be at least 4130 or greater");
                }

                initialize_logger();
                print_welcome();

                let _node = Node::<Testnet2, Client>::new(node_port, rpc_port, (node_port as u16 - 4130) as u8, None).await?;
            }
        }

        Ok(())
    }
}
