// Copyright (C) 2019-2020 Aleo Systems Inc.
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
    miner::MinerInstance,
};
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkos_dpc::base_dpc::{instantiated::Components, parameters::PublicParameters, BaseDPCComponents};
use snarkos_errors::node::NodeError;
use snarkos_models::algorithms::{CRH, SNARK};
use snarkos_network::{environment::Environment, Server};
use snarkos_objects::{AccountAddress, Network};
use snarkos_posw::PoswMarlin;
use snarkos_rpc::start_rpc_server;
use snarkos_utilities::{to_bytes, ToBytes};

use std::{
    net::SocketAddr,
    str::FromStr,
    sync::{Arc, Mutex, RwLock},
};
use tokio::{
    runtime::Builder,
    stream::{self, StreamExt},
};
use tracing_futures::Instrument;
use tracing_subscriber::EnvFilter;

fn initialize_logger(config: &Config) {
    match config.node.verbose {
        0 => {}
        verbosity => {
            match verbosity {
                1 => std::env::set_var("RUST_LOG", "trace"),
                2 => std::env::set_var("RUST_LOG", "debug"),
                3 => std::env::set_var("RUST_LOG", "trace"),
                _ => std::env::set_var("RUST_LOG", "info"),
            };

            // disable undesirable logs
            let filter = EnvFilter::from_default_env().add_directive("tokio_reactor=off".parse().unwrap());

            // initialize tracing
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_target(false)
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
/// 1. Creates new storage database or uses existing.
/// 2. Creates new memory pool or uses existing from storage.
/// 3. Creates consensus parameters.
/// 4. Creates network server.
/// 5. Starts rpc server thread.
/// 6. Starts miner thread.
/// 7. Starts network server listener.
///
async fn start_server(config: Config) -> anyhow::Result<()> {
    initialize_logger(&config);

    print_welcome(&config);

    let address = format! {"{}:{}", config.node.ip, config.node.port};
    let socket_address = address.parse::<SocketAddr>()?;

    let mut path = config.node.dir;
    path.push(&config.node.db);
    let storage = MerkleTreeLedger::open_at_path(path.clone())?;
    // let storage = Arc::new(MerkleTreeLedger::open_at_path(path.clone())?);

    let memory_pool = Arc::new(Mutex::new(MemoryPool::from_storage(&storage)?));

    info!("Loading Aleo parameters...");
    let parameters = PublicParameters::<Components>::load(!config.miner.is_miner)?;
    info!("Loading complete.");

    // Fetch the set of valid inner circuit IDs.
    let inner_snark_vk: <<Components as BaseDPCComponents>::InnerSNARK as SNARK>::VerificationParameters =
        parameters.inner_snark_parameters.1.clone().into();
    let inner_snark_id = parameters
        .system_parameters
        .inner_snark_verification_key_crh
        .hash(&to_bytes![inner_snark_vk]?)?;

    let authorized_inner_snark_ids = vec![to_bytes![inner_snark_id]?];

    // Set the initial consensus parameters.
    let consensus = ConsensusParameters {
        max_block_size: 1_000_000_000usize,
        max_nonce: u32::max_value(),
        target_block_time: 10i64,
        network_id: Network::from_network_id(config.aleo.network_id),
        verifier: PoswMarlin::verify_only().expect("could not instantiate PoSW verifier"),
        authorized_inner_snark_ids,
    };

    let mut environment = Environment::new(
        Arc::new(RwLock::new(storage)),
        memory_pool.clone(),
        Arc::new(consensus.clone()),
        Arc::new(parameters.clone()),
        Some(socket_address),
        config.p2p.min_peers,
        config.p2p.max_peers,
        100,
        config.p2p.mempool_interval,
        config.p2p.bootnodes.clone(),
        config.node.is_bootnode,
        config.miner.is_miner,
    )?;

    // Construct the server instance. Note this does not start the server.
    // This is done early on, so that the local address can be discovered
    // before any other object (miner, RPC) needs to use it.
    let mut server = Server::new(environment.clone()).await?;

    // Establish the address of the server.
    server.establish_address().await?;
    environment.set_local_address(server.local_address().unwrap());

    // Start the miner task if mining configuration is enabled.
    if config.miner.is_miner {
        match AccountAddress::<Components>::from_str(&config.miner.miner_address) {
            Ok(miner_address) => {
                MinerInstance::new(
                    miner_address,
                    consensus.clone(),
                    parameters.clone(),
                    environment.clone(),
                    server.clone(),
                )
                .spawn();
            }
            Err(_) => info!(
                "Miner not started. Please specify a valid miner address in your ~/.snarkOS/config.toml file or by using the --miner-address option in the CLI."
            ),
        }
    }

    // Start RPC thread, if the RPC configuration is enabled.
    if config.rpc.json_rpc {
        info!("Loading Aleo parameters for RPC...");
        let proving_parameters = PublicParameters::<Components>::load(!config.miner.is_miner)?;
        info!("Loading complete.");

        // Open a secondary storage instance to prevent resource sharing and bottle-necking.
        let secondary_storage = Arc::new(RwLock::new(MerkleTreeLedger::open_secondary_at_path(path.clone())?));

        start_rpc_server(
            config.rpc.port,
            secondary_storage,
            path.to_path_buf(),
            proving_parameters,
            environment,
            consensus,
            memory_pool,
            server.clone(),
            config.rpc.username,
            config.rpc.password,
        )
        .await;
    }

    // Start the server
    server.start_services().await?;

    stream::pending::<()>().next().await;

    Ok(())
}

fn main() -> Result<(), NodeError> {
    let arguments = ConfigCli::new();
    let config: Config = ConfigCli::parse(&arguments)?;
    let node_span = debug_span!("node");

    Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(4 * 1024 * 1024)
        .build()?
        .block_on(start_server(config).instrument(node_span))?;

    Ok(())
}
