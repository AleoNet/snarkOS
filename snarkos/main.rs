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
use snarkos_storage::{export_canon_blocks, key_value::KeyValueStore, RocksDb, SerialBlock, Storage, VMBlock};
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
/// 1. Creates new storage database or uses existing.
/// 2. Creates new memory pool or uses existing from storage.
/// 3. Creates sync parameters.
/// 4. Creates network server.
/// 5. Starts rpc server thread.
/// 6. Starts miner thread.
/// 7. Starts network server listener.
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

    info!("Loading storage at '{}'...", path.to_str().unwrap_or_default());
    let storage = Arc::new(KeyValueStore::new(RocksDb::open(&path)?));
    info!("Storage finished loading");

    // Construct the node instance. Note this does not start the network services.
    // This is done early on, so that the local address can be discovered
    // before any other object (miner, RPC) needs to use it.
    let mut node = Node::new(node_config, storage.clone()).await?;

    // For extra safety, validate storage too if a trim is requested.
    // if config.storage.validate || config.storage.trim {
    //     let now = std::time::Instant::now();
    //     storage
    //         .validate(None, snarkos_storage::validator::FixMode::Everything)
    //         .await;
    //     info!("Storage validated in {}ms", now.elapsed().as_millis());
    //     if !config.storage.trim {
    //         return Ok(());
    //     }
    // }

    if config.storage.trim {
        let now = std::time::Instant::now();
        // There shouldn't be issues after validation, but if there are, ignore them.
        let _ = snarkos_storage::trim(storage.clone()).await;
        info!("Storage trimmed in {}ms", now.elapsed().as_millis());
        return Ok(());
    }

    if let Some(limit) = config.storage.export {
        let mut export_path = path.clone();
        export_path.push("canon_blocks");

        let limit = if limit == 0 { None } else { Some(limit) };

        let now = std::time::Instant::now();
        match export_canon_blocks(storage.clone(), limit, &export_path).await {
            Ok(num_exported) => {
                info!(
                    "{} canon blocks exported to {} in {}ms",
                    num_exported,
                    export_path.display(),
                    now.elapsed().as_millis()
                );
            }
            Err(e) => error!("Couldn't export canon blocks to {}: {}", export_path.display(), e),
        }
    }

    // Enable the sync layer.
    {
        let memory_pool = MemoryPool::new(); // from_storage(&storage).await?;

        debug!("Loading Aleo parameters...");
        let dpc = <Testnet1DPC as DPCScheme<DeserializedLedger<'_, Components>>>::load(!config.miner.is_miner)?;
        info!("Loaded Aleo parameters");

        // Fetch the set of valid inner circuit IDs.
        let inner_snark_vk: <<Components as Testnet1Components>::InnerSNARK as SNARK>::VerifyingKey =
            dpc.inner_snark_parameters.1.clone().into();
        let inner_snark_id = dpc
            .system_parameters
            .inner_circuit_id_crh
            .hash(&to_bytes_le![inner_snark_vk]?)?;

        let authorized_inner_snark_ids = vec![to_bytes_le![inner_snark_id]?];

        // Set the initial sync parameters.
        let consensus_params = ConsensusParameters {
            max_block_size: 1_000_000_000usize,
            max_nonce: u32::max_value(),
            target_block_time: 10i64,
            network_id: Network::from_id(config.aleo.network_id),
            verifier: PoswMarlin::verify_only().expect("could not instantiate PoSW verifier"),
            authorized_inner_snark_ids,
        };

        let ledger_parameters = {
            type Parameters = <Components as Testnet1Components>::MerkleParameters;
            let parameters: <<Parameters as MerkleParameters>::H as CRH>::Parameters =
                FromBytes::read_le(&LedgerMerkleTreeParameters::load_bytes()?[..])?;
            let crh = <Parameters as MerkleParameters>::H::from(parameters);
            Arc::new(Parameters::from(crh))
        };
        info!("Loading Ledger");
        let ledger_digests = storage.get_ledger_digests().await?;
        let commitments = storage.get_commitments().await?;
        let serial_numbers = storage.get_serial_numbers().await?;
        let memos = storage.get_memos().await?;
        info!("Initializing Ledger");
        let ledger = DynLedger(Box::new(MerkleLedger::new(
            ledger_parameters,
            &ledger_digests[..],
            &commitments[..],
            &serial_numbers[..],
            &memos[..],
        )?));

        let genesis_block: Block<Testnet1Transaction> = FromBytes::read_le(GenesisBlock::load_bytes().as_slice())?;
        let genesis_block: SerialBlock = <Block<Testnet1Transaction> as VMBlock>::serialize(&genesis_block)?;

        let consensus = Consensus::new(
            consensus_params,
            Arc::new(dpc),
            genesis_block,
            ledger,
            storage.clone(),
            memory_pool,
        );
        info!("Loaded Ledger");

        if config.storage.scan_for_forks {
            consensus.scan_forks().await?;
        }

        if let Some(import_path) = config.storage.import {
            info!("Importing canon blocks from {}", import_path.display());

            let now = std::time::Instant::now();
            let mut blocks = std::io::Cursor::new(std::fs::read(import_path)?);

            let mut processed = 0usize;
            let mut imported = 0usize;
            while let Ok(block) = Block::<Testnet1Transaction>::read_le(&mut blocks) {
                let block = <Block<Testnet1Transaction> as VMBlock>::serialize(&block)?;
                // Skip possible duplicate blocks etc.
                if consensus.receive_block(block).await {
                    imported += 1;
                }
                processed += 1;
            }

            info!(
                "Processed {} canon blocks ({} imported) in {}ms",
                processed,
                imported,
                now.elapsed().as_millis()
            );
        }

        let sync = Sync::new(
            consensus,
            config.miner.is_miner,
            Duration::from_secs(config.p2p.block_sync_interval.into()),
            Duration::from_secs(config.p2p.mempool_sync_interval.into()),
        );

        node.set_sync(sync);
    }

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

    // Start the miner task if mining configuration is enabled.
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    if config.miner.is_miner {
        match Address::<Components>::from_str(&config.miner.miner_address) {
            Ok(miner_address) => {
                let handle = MinerInstance::new(miner_address, node.clone()).spawn().await?;
                node.register_task(handle);
            }
            Err(_) => info!(
                "Miner not started. Please specify a valid miner address in your ~/.snarkOS/config.toml file or by using the --miner-address option in the CLI."
            ),
        }
    }

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
