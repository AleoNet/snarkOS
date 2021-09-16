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

use snarkos::{
    cli::CLI,
    config::{Config, ConfigCli},
    display::{initialize_logger, print_welcome},
    errors::NodeError,
    init::{init_ephemeral_storage, init_node, init_rpc},
};
use snarkos_network::NodeType;

use tokio::runtime;

///
/// Builds a node from configuration parameters.
///
/// 1. Creates network server.
/// 2. Starts rpc server thread.
/// 3. Starts the network listener.
///
async fn start_server(config: Config) -> anyhow::Result<()> {
    initialize_logger(&config);

    print_welcome(&config);

    let storage = init_ephemeral_storage()?;

    // Construct the node instance. Note this does not start the network services.
    // This is done early on, so that the local address can be discovered
    // before any other object (RPC) needs to use it.
    let node = init_node(&config, storage.clone()).await?;

    // Initialize metrics framework.
    node.initialize_metrics().await?;

    // Start listening for incoming connections.
    node.listen().await?;

    // Start RPC thread, if the RPC configuration is enabled.
    if config.rpc.json_rpc {
        init_rpc(&config, node.clone(), storage)?;
    }

    // Start the network services.
    node.start_services().await;

    std::future::pending::<()>().await;

    Ok(())
}

fn main() -> Result<(), NodeError> {
    let arguments = ConfigCli::args();

    let mut config: Config = ConfigCli::parse(&arguments)?;
    config.node.kind = NodeType::Beacon;
    config.check().map_err(|e| NodeError::Message(e.to_string()))?;

    let runtime = runtime::Builder::new_multi_thread().enable_all().build()?;

    runtime.block_on(start_server(config))?;

    Ok(())
}
