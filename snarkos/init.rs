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
    cli::CLI,
    config::{Config, ConfigCli},
    display::{initialize_logger, print_welcome},
    errors::NodeError,
};
use snarkos_consensus::{Consensus, ConsensusParameters, DeserializedLedger, DynLedger, MemoryPool, MerkleLedger};
use snarkos_network::{config::Config as NodeConfig, MinerInstance, Node, NodeType, Sync};
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

use std::{fs, net::SocketAddr, path::PathBuf, str::FromStr, sync::Arc, time::Duration};

use tokio::runtime;

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

pub async fn init_node(config: &Config, storage: Option<DynStorage>) -> anyhow::Result<Node> {
    let address = format!("{}:{}", config.node.ip, config.node.port);
    let desired_address = address.parse::<SocketAddr>()?;

    let node_config = NodeConfig::new(
        None,
        config.node.kind,
        desired_address,
        config.p2p.min_peers,
        config.p2p.max_peers,
        config.p2p.beacons.clone(),
        // Set sync intervals for peers.
        Duration::from_secs(config.p2p.peer_sync_interval.into()),
    )?;

    let node = Node::new(node_config, storage).await?;

    Ok(node)
}

// pub async fn init_sync(config: &Config)
