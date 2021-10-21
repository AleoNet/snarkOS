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

use crate::config::Config;
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

use std::{fs, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

pub fn init_ephemeral_storage() -> anyhow::Result<DynStorage> {
    Ok(Arc::new(AsyncStorage::new(SqliteStorage::new_ephemeral()?)))
}

pub async fn init_storage(config: &Config) -> anyhow::Result<Option<DynStorage>> {
    let mut path = config.node.dir.clone();
    path.push(&config.node.db);

    if !path.exists() {
        fs::create_dir(&path)?;
    }

    info!("Loading storage at '{}'...", path.to_str().unwrap_or_default());
    let storage: DynStorage = {
        let mut sqlite_path = path.clone();
        sqlite_path.push("sqlite.db");

        if config.storage.validate {
            error!("validator not implemented for sqlite");
            // FIXME: this should probably be an error, perhaps handled at the CLI level.
            return Ok(None);
        }

        Arc::new(AsyncStorage::new(SqliteStorage::new(&sqlite_path)?))
    };

    if storage.canon().await?.block_height == 0 {
        let mut rocks_identity_path = path.clone();
        rocks_identity_path.push("IDENTITY");
        if rocks_identity_path.exists() {
            info!("Empty sqlite DB with existing rocksdb found, migrating...");
            let rocks_storage = RocksDb::open(&path)?;
            let rocks_storage: DynStorage = Arc::new(AsyncStorage::new(KeyValueStore::new(rocks_storage)));

            snarkos_storage::migrate(&rocks_storage, &storage).await?;
        }
    }

    if let Some(max_head) = config.storage.max_head {
        let canon_next = storage.get_block_hash(max_head + 1).await?;
        if let Some(canon_next) = canon_next {
            storage.decommit_blocks(&canon_next).await?;
        }
    }

    if config.storage.trim {
        let now = std::time::Instant::now();
        // There shouldn't be issues after validation, but if there are, ignore them.
        let _ = snarkos_storage::trim(storage.clone()).await;
        info!("Storage trimmed in {}ms", now.elapsed().as_millis());
        return Ok(None);
    }

    info!("Storage is ready");

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

    Ok(Some(storage))
}

pub async fn init_sync(config: &Config, storage: DynStorage) -> anyhow::Result<Sync> {
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
    let ledger_digests = storage.get_ledger_digests(0).await?;
    let commitments = storage.get_commitments(0).await?;
    let serial_numbers = storage.get_serial_numbers(0).await?;
    let memos = storage.get_memos(0).await?;
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
        !config.storage.no_recanonize,
    )
    .await;
    info!("Loaded Ledger");

    if config.storage.scan_for_forks {
        storage
            .scan_forks(snarkos_consensus::OLDEST_FORK_THRESHOLD as u32)
            .await?;
    }

    if let Some(import_path) = &config.storage.import {
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

    Ok(sync)
}

pub async fn init_node(config: &Config, storage: DynStorage) -> anyhow::Result<Node> {
    let address = format!("{}:{}", config.node.ip, config.node.port);
    let desired_address = address.parse::<SocketAddr>()?;

    let node_config = NodeConfig::new(
        None,
        config.node.kind,
        desired_address,
        config.p2p.min_peers,
        config.p2p.max_peers,
        config.p2p.beacons.clone(),
        config.p2p.sync_providers.clone(),
        // Set sync intervals for peers.
        Duration::from_secs(config.p2p.peer_sync_interval.into()),
    )?;

    let node = Node::new(node_config, storage).await?;

    Ok(node)
}

pub fn init_rpc(config: &Config, node: Node, storage: DynStorage) -> anyhow::Result<()> {
    let rpc_address = format!("{}:{}", config.rpc.ip, config.rpc.port)
        .parse()
        .expect("Invalid RPC server address!");

    let rpc_handle = start_rpc_server(
        rpc_address,
        storage,
        node.clone(),
        config.rpc.username.clone(),
        config.rpc.password.clone(),
    );
    node.register_task(rpc_handle);

    info!("Listening for RPC requests on port {}", config.rpc.port);

    Ok(())
}

pub fn init_miner(config: &Config, node: Node) {
    match Address::<Components>::from_str(&config.miner.miner_address) {
        Ok(miner_address) => {
            let handle = MinerInstance::new(miner_address, node.clone()).spawn();
            node.register_task(handle);
        }
        Err(_) => info!(
            "Miner not started. Please specify a valid miner address in your ~/.snarkOS/config.toml file or by using the --miner-address option in the CLI."
        ),
    }
}
