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
    display::render_welcome,
    errors::NodeError,
};
use snarkos_consensus::{Consensus, ConsensusParameters, DeserializedLedger, DynLedger, MemoryPool, MerkleLedger};
use snarkos_network::{config::Config as NodeConfig, MinerInstance, Node, Sync};
use snarkos_rpc::start_rpc_server;
use snarkos_storage::{
    export_canon_blocks,
    key_value::KeyValueStore,
    AsyncStorage,
    DynStorage,
    RocksDb,
    SerialBlock,
    SqliteStorage,
    VMBlock,
};

use snarkvm_algorithms::{MerkleParameters, CRH, SNARK};
use snarkvm_dpc::{
    testnet1::{
        instantiated::{Components, Testnet1DPC, Testnet1Transaction},
        Testnet1Components,
    },
    Address,
    Block,
    DPCScheme,
    Network,
};
use snarkvm_parameters::{testnet1::GenesisBlock, Genesis, LedgerMerkleTreeParameters, Parameter};
use snarkvm_posw::PoswMarlin;
use snarkvm_utilities::{to_bytes_le, FromBytes, ToBytes};

use std::{net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

use tokio::runtime;
use tracing_subscriber::EnvFilter;

fn initialize_logger(config: &Config) {
    match config.node.verbose {
        0 => {}
        verbosity => {
            match verbosity {
                1 => std::env::set_var("RUST_LOG", "info"),
                2 => std::env::set_var("RUST_LOG", "debug"),
                3 | 4 => std::env::set_var("RUST_LOG", "trace"),
                _ => std::env::set_var("RUST_LOG", "info"),
            };

            // disable undesirable logs
            let filter = EnvFilter::from_default_env().add_directive("mio=off".parse().unwrap());

            // initialize tracing
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_target(config.node.verbose == 4)
                .init();
        }
    }
}

fn print_welcome(config: &Config) {
    println!("{}", render_welcome(config));
}

///
/// Builds a node from configuration parameters.
///
/// 1. Creates network server.
/// 2. Starts rpc server thread.
/// 3. Starts network server listener.
///
async fn start_server(config: Config) -> anyhow::Result<()> {
    initialize_logger(&config);

    print_welcome(&config);

    let address = format!("{}:{}", config.node.ip, config.node.port);
    let desired_address = address.parse::<SocketAddr>()?;

    let mut path = config.node.dir.clone();
    path.push(&config.node.db);

    let node_config = NodeConfig::new(
        desired_address,
        config.p2p.min_peers,
        config.p2p.max_peers,
        config.p2p.bootnodes.clone(),
        config.node.is_bootnode,
        config.node.is_crawler,
        // Set sync intervals for peers, blocks and transactions (memory pool).
        Duration::from_secs(config.p2p.peer_sync_interval.into()),
    )?;

    // Create the storage instance; currently unused but may be useful to store addresses.
    // TODO: remove?
    info!("Loading storage at '{}'...", path.to_str().unwrap_or_default());
    let storage: DynStorage = {
        let mut sqlite_path = path.clone();
        sqlite_path.push("sqlite.db");

        if config.storage.validate {
            error!("validator not implemented for sqlite");
            return Ok(());
        }

        Arc::new(AsyncStorage::new(SqliteStorage::new(&sqlite_path)?))
    };

    info!("Storage is ready");

    // Construct the node instance. Note this does not start the network services.
    // This is done early on, so that the local address can be discovered
    // before any other object (miner, RPC) needs to use it.
    let mut node = Node::new(node_config, storage.clone()).await?;

    // Initialize metrics framework
    node.initialize_metrics().await?;

    // Start listening for incoming connections.
    node.listen().await?;

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

    // Start the network services
    node.start_services().await;

    std::future::pending::<()>().await;

    Ok(())
}

fn main() -> Result<(), NodeError> {
    let arguments = ConfigCli::args();

    let config: Config = ConfigCli::parse(&arguments)?;
    config.check().map_err(|e| NodeError::Message(e.to_string()))?;

    let runtime = runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .build()?;

    runtime.block_on(start_server(config))?;

    Ok(())
}
