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
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkos_network::{environment::Environment, Consensus, MinerInstance, Node};
use snarkos_rpc::start_rpc_server;
use snarkos_storage::LedgerStorage;
use snarkvm_algorithms::{CRH, SNARK};
use snarkvm_dpc::{
    base_dpc::{instantiated::Components, parameters::PublicParameters, BaseDPCComponents},
    AccountAddress,
};
use snarkvm_objects::{Network, Storage};
use snarkvm_posw::PoswMarlin;
use snarkvm_utilities::{to_bytes, ToBytes};

use std::{net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

use parking_lot::Mutex;
use tokio::runtime::Builder;
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

    let mut environment = Environment::new(
        Some(socket_address),
        config.p2p.min_peers,
        config.p2p.max_peers,
        config.p2p.bootnodes.clone(),
        config.node.is_bootnode,
        // Set sync intervals for peers, blocks and transactions (memory pool).
        Duration::from_secs(config.p2p.peer_sync_interval.into()),
    )?;

    // Construct the node instance. Note this does not start the network services.
    // This is done early on, so that the local address can be discovered
    // before any other object (miner, RPC) needs to use it.
    let mut node = Node::new(environment.clone()).await?;

    let is_storage_in_memory = LedgerStorage::IN_MEMORY;

    let storage = if is_storage_in_memory {
        Arc::new(MerkleTreeLedger::<LedgerStorage>::new_empty(
            None::<std::path::PathBuf>,
        )?)
    } else {
        Arc::new(MerkleTreeLedger::<LedgerStorage>::open_at_path(path.clone())?)
    };

    // Enable the consensus layer if the node is not a bootstrapper.
    if !config.node.is_bootnode {
        let memory_pool = Mutex::new(MemoryPool::from_storage(&storage)?);

        info!("Loading Aleo parameters...");
        let dpc_parameters = PublicParameters::<Components>::load(!config.miner.is_miner)?;
        info!("Loading complete.");

        // Fetch the set of valid inner circuit IDs.
        let inner_snark_vk: <<Components as BaseDPCComponents>::InnerSNARK as SNARK>::VerificationParameters =
            dpc_parameters.inner_snark_parameters.1.clone().into();
        let inner_snark_id = dpc_parameters
            .system_parameters
            .inner_snark_verification_key_crh
            .hash(&to_bytes![inner_snark_vk]?)?;

        let authorized_inner_snark_ids = vec![to_bytes![inner_snark_id]?];

        // Set the initial consensus parameters.
        let consensus_params = ConsensusParameters {
            max_block_size: 1_000_000_000usize,
            max_nonce: u32::max_value(),
            target_block_time: 10i64,
            network_id: Network::from_network_id(config.aleo.network_id),
            verifier: PoswMarlin::verify_only().expect("could not instantiate PoSW verifier"),
            authorized_inner_snark_ids,
        };

        let consensus = Arc::new(snarkos_consensus::Consensus {
            ledger: Arc::clone(&storage),
            memory_pool,
            parameters: consensus_params,
            public_parameters: dpc_parameters,
        });

        let consensus = Consensus::new(
            node.clone(),
            consensus,
            config.miner.is_miner,
            Duration::from_secs(config.p2p.block_sync_interval.into()),
            Duration::from_secs(config.p2p.mempool_interval.into()),
        );

        node.set_consensus(consensus);
    };

    // Establish the address of the node.
    node.establish_address().await?;
    environment.set_local_address(node.local_address().unwrap());

    // Start the miner task if mining configuration is enabled.
    if config.miner.is_miner {
        match AccountAddress::<Components>::from_str(&config.miner.miner_address) {
            Ok(miner_address) => {
                MinerInstance::new(miner_address, node.clone()).spawn();
            }
            Err(_) => info!(
                "Miner not started. Please specify a valid miner address in your ~/.snarkOS/config.toml file or by using the --miner-address option in the CLI."
            ),
        }
    }

    // Start RPC thread, if the RPC configuration is enabled.
    if config.rpc.json_rpc {
        let secondary_storage = if is_storage_in_memory {
            // In-memory storage doesn't require a secondary instance.
            storage
        } else {
            // Open a secondary storage instance to prevent resource sharing and bottle-necking.
            Arc::new(MerkleTreeLedger::open_secondary_at_path(path.clone())?)
        };

        start_rpc_server(
            config.rpc.port,
            secondary_storage,
            node.clone(),
            config.rpc.username,
            config.rpc.password,
        )
        .await;
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
    let node_span = debug_span!("node");

    Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(4 * 1024 * 1024)
        .build()?
        .block_on(start_server(config).instrument(node_span))?;

    Ok(())
}
