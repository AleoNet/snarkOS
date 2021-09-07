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

#[macro_use]
extern crate tracing;

use snarkos::{
    cli::CLI,
    config::{Config, ConfigCli},
    display::{initialize_logger, print_welcome},
    errors::NodeError,
};
use snarkos_network::{config::Config as NodeConfig, Node};
use snarkos_rpc::start_rpc_server;
use snarkos_storage::{AsyncStorage, SqliteStorage};

use std::{net::SocketAddr, sync::Arc, time::Duration};

use tokio::runtime;

///
/// Builds a node from configuration parameters.
///
/// 1. Creates network server.
/// 2. Starts rpc server thread.
///
async fn start_server(config: Config) -> anyhow::Result<()> {
    initialize_logger(&config);

    print_welcome(&config);

    let address = format!("{}:{}", config.node.ip, config.node.port);
    let desired_address = address.parse::<SocketAddr>()?;

    let node_config = NodeConfig::new(
        None,
        desired_address,
        config.p2p.min_peers,
        config.p2p.max_peers,
        config.p2p.bootnodes.clone(),
        false, // is_bootnode
        true,  // is crawler
        // Set sync intervals for peers.
        Duration::from_secs(config.p2p.peer_sync_interval.into()),
    )?;

    // Construct the node instance. Note this does not start the network services.
    // This is done early on, so that the local address can be discovered
    // before any other object (RPC) needs to use it.

    let storage = Arc::new(AsyncStorage::new(SqliteStorage::new_ephemeral().unwrap()));

    let node = Node::new(node_config, storage.clone()).await?;

    // Initialize metrics framework.
    node.initialize_metrics().await?;

    // Start RPC thread, if the RPC configuration is enabled.
    if config.rpc.json_rpc {
        let rpc_address = format!("{}:{}", config.rpc.ip, config.rpc.port)
            .parse()
            .expect("Invalid RPC server address!");

        let rpc_handle = start_rpc_server(
            rpc_address,
            storage,
            node.clone(),
            config.rpc.username,
            config.rpc.password,
        );
        node.register_task(rpc_handle);

        info!("Listening for RPC requests on port {}", config.rpc.port);
    }

    // Start the network services.
    node.start_services().await;

    std::future::pending::<()>().await;

    Ok(())
}

fn main() -> Result<(), NodeError> {
    let arguments = ConfigCli::args();

    let mut config: Config = ConfigCli::parse(&arguments)?;
    config.is_crawler(true);
    config.check().map_err(|e| NodeError::Message(e.to_string()))?;

    let runtime = runtime::Builder::new_multi_thread().enable_all().build()?;

    runtime.block_on(start_server(config))?;

    Ok(())
}
