// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

#[cfg(not(feature = "test"))]
use aleo_std::aleo_dir;
use mysten_network::multiaddr::Multiaddr;
use narwhal_config::{Committee, CommitteeBuilder, WorkerCache, WorkerIndex, WorkerInfo};
use narwhal_crypto::{EncodeDecodeBase64, KeyPair as NarwhalKeyPair, NetworkKeyPair, PublicKey};
use rand::prelude::ThreadRng;
use tracing::*;

// These ports are used by tests and in dev mode.
mod test_ports {
    use std::sync::atomic::AtomicU16;

    // The non-registered port range for primaries (27 slots).
    pub(super) const PRIMARY_FIRST_PORT: u16 = 1030;
    pub(super) const PRIMARY_LAST_PORT: u16 = 1057;

    // The non-registered network port range for workers (27 slots).
    pub(super) const WORKER_FIRST_PORT_NET: u16 = 1242;
    pub(super) const WORKER_LAST_PORT_NET: u16 = 1269;

    // The non-registered transaction port range for workers (53 slots).
    pub(super) const WORKER_FIRST_PORT_TX: u16 = 1360;
    pub(super) const WORKER_LAST_PORT_TX: u16 = 1413;

    // Offsets to use when creating multiple primaries and workers.
    pub(super) static PRIMARY_PORT_OFFSET: AtomicU16 = AtomicU16::new(0);
    pub(super) static WORKER_PORT_OFFSET_NET: AtomicU16 = AtomicU16::new(0);
    pub(super) static WORKER_PORT_OFFSET_TX: AtomicU16 = AtomicU16::new(0);
}
use test_ports::*;

// A collection of values required to create a full primary.
pub struct PrimarySetup {
    pub stake: u64,
    pub address: Multiaddr,
    pub keypair: NarwhalKeyPair,
    pub network_keypair: NetworkKeyPair,
    pub workers: Vec<WorkerSetup>,
}

impl PrimarySetup {
    // TODO: maybe improve the UX here a little bit?
    pub fn new(
        primary_addr: Option<Multiaddr>,
        stake: u64,
        worker_addrs: Vec<(Multiaddr, Multiaddr)>, // (network_addr, tx_addr)
        rng: &mut ThreadRng,
    ) -> Self {
        if worker_addrs.len() > 1 {
            panic!(
                "Running multiple workers on a single machine is currently unsupported;\
                    the bullshark-bft crate would need to be adjusted for that feature."
            );
        }

        // If no worker addresses are provided, create one using defaults; otherwise, iterate.
        let workers = if worker_addrs.is_empty() {
            vec![WorkerSetup::new(None, rng)]
        } else {
            worker_addrs.into_iter().map(|addrs| WorkerSetup::new(Some(addrs), rng)).collect()
        };

        // Default to a local network address if one is not provided.
        let address = if let Some(addr) = primary_addr {
            addr
        } else {
            let primary_port = PRIMARY_FIRST_PORT + PRIMARY_PORT_OFFSET.fetch_add(1, Ordering::SeqCst);
            if primary_port > PRIMARY_LAST_PORT {
                warn!("Primary port is running into registered range ({primary_port}).");
            }

            format!("/ip4/127.0.0.1/udp/{primary_port}").parse().unwrap()
        };

        Self {
            stake,
            address,
            keypair: NarwhalKeyPair::new(rng).expect("Failed to generate primary keypair."),
            network_keypair: NetworkKeyPair::generate(rng),
            workers,
        }
    }
}

// A collection of values required to create a full worker.
pub struct WorkerSetup {
    pub address: Multiaddr,
    pub tx_address: Multiaddr,
    pub network_keypair: NetworkKeyPair,
}

impl WorkerSetup {
    fn new(addrs: Option<(Multiaddr, Multiaddr)>, rng: &mut ThreadRng) -> Self {
        // Default to local network addresses if none are provided.
        let (address, tx_address) = if let Some(addrs) = addrs {
            addrs
        } else {
            let worker_port_net = WORKER_FIRST_PORT_NET + WORKER_PORT_OFFSET_NET.fetch_add(1, Ordering::SeqCst);
            if worker_port_net > WORKER_LAST_PORT_NET {
                warn!("Worker network port is running into registered range ({worker_port_net}).");
            }
            let address = format!("/ip4/127.0.0.1/udp/{worker_port_net}").parse().unwrap();

            let worker_port_tx = WORKER_FIRST_PORT_TX + WORKER_PORT_OFFSET_TX.fetch_add(1, Ordering::SeqCst);
            if worker_port_tx > WORKER_LAST_PORT_TX {
                warn!("Worker transaction port is running into registered range ({worker_port_tx}).");
            }
            let tx_address = format!("/ip4/127.0.0.1/tcp/{worker_port_tx}/http").parse().unwrap();

            (address, tx_address)
        };

        Self { address, tx_address, network_keypair: NetworkKeyPair::generate(rng) }
    }
}

// A collection of values capable of generating the entire BFT committee.
pub struct CommitteeSetup {
    pub primaries: BTreeMap<PublicKey, PrimarySetup>,
    pub epoch: u64,
}

impl CommitteeSetup {
    pub fn new(primaries: Vec<PrimarySetup>, epoch: u64) -> Self {
        Self { primaries: primaries.into_iter().map(|ps| (ps.keypair.public().clone(), ps)).collect(), epoch }
    }

    // Generates a Committee.
    pub fn generate_committee(&self) -> Committee {
        let mut committee_builder = CommitteeBuilder::new(0);
        for (primary_public, primary) in &self.primaries {
            committee_builder = committee_builder.add_authority(
                primary_public.clone(),
                primary.stake,
                primary.address.clone(),
                primary.network_keypair.public().clone(),
            );
        }
        committee_builder.build()
    }

    // Generates a WorkerCache.
    pub fn generate_worker_cache(&self) -> WorkerCache {
        #[allow(clippy::mutable_key_type)]
        let mut workers = BTreeMap::default();
        for (primary_public, primary) in &self.primaries {
            let mut worker_index = BTreeMap::default();
            for (worker_id, worker) in primary.workers.iter().enumerate() {
                let worker_info = WorkerInfo {
                    name: worker.network_keypair.public().clone(),
                    transactions: worker.tx_address.clone(),
                    worker_address: worker.address.clone(),
                };

                worker_index.insert(worker_id as u32, worker_info);
            }
            let worker_index = WorkerIndex(worker_index);
            workers.insert(primary_public.clone(), worker_index);
        }

        WorkerCache { workers, epoch: self.epoch }
    }

    // Persists the committee setup to the filesystem.
    pub fn write_files(&self, dev: bool) {
        fn dev_subpath(dev: bool) -> &'static str {
            if dev { ".dev/" } else { "" }
        }

        // Generate the common config.
        let committee = self.generate_committee();
        let worker_cache = self.generate_worker_cache();

        // Check if the base path exists.
        let base_path = format!("{}/node/bft-consensus/committee/{}", workspace_dir(), dev_subpath(dev));
        if fs::metadata(&base_path).is_err() {
            debug!("Creating missing directory {base_path}");
            fs::create_dir_all(&base_path)
                .unwrap_or_else(|error| panic!("Couldn't create the missing {base_path}: {error:?}"));
        }

        // Write the committee file to the filesystem.
        let committee_path = format!("{base_path}.committee.json");
        let committee_json = serde_json::to_string_pretty(&committee).unwrap();
        fs::write(committee_path, committee_json).unwrap();

        // Write the worker cache file to the filesystem.
        let workers_path = format!("{base_path}.workers.json");
        let workers_json = serde_json::to_string_pretty(&worker_cache).unwrap();
        fs::write(workers_path, workers_json).unwrap();

        // Write the primary and worker files to the filesystem.
        for (primary_id, (_, primary)) in self.primaries.iter().enumerate() {
            // Base64-encode the primary keys.
            let primary_key_encoded = primary.keypair.encode_base64();
            let primary_network_key_encoded = primary.network_keypair.encode_base64();

            // Write the encoded primary keys to the filesystem.
            let primary_key_path = format!("{base_path}.primary-{primary_id}-key.json");
            fs::write(primary_key_path, primary_key_encoded).unwrap();
            let primary_network_key_path = format!("{base_path}.primary-{primary_id}-network.json");
            fs::write(primary_network_key_path, primary_network_key_encoded).unwrap();

            for (worker_id, worker) in primary.workers.iter().enumerate() {
                // Base64-encode the worker network key.
                let worker_network_key_encoded = worker.network_keypair.encode_base64();

                // Write the encoded worker key to the filesystem.
                let worker_network_key_path = format!("{base_path}.worker-{primary_id}-{worker_id}-network.json");
                fs::write(worker_network_key_path, worker_network_key_encoded).unwrap();
            }
        }
    }
}

// Returns the base path for the BFT storage files.
#[cfg(not(feature = "test"))]
fn base_storage_path(dev: Option<u16>) -> PathBuf {
    // Retrieve the starting directory.
    match dev.is_some() {
        // In development mode, the ledger is stored in the root directory of the repository.
        true => workspace_dir().into(),
        // In production mode, the ledger is stored in the `~/.aleo/` directory.
        false => aleo_dir(),
    }
}

// Returns the base path for the BFT storage files.
#[cfg(feature = "test")]
fn base_storage_path(_dev: Option<u16>) -> PathBuf {
    // The call to `into_path` causes the directory to not be deleted afterwards,
    // but it resides in the system's temporary file directory, so it gets removed
    // soon regardless.
    tempfile::TempDir::new().unwrap().into_path()
}

// Returns the path for the primary-related BFT files.
pub(crate) fn primary_storage_dir(network: u16, dev: Option<u16>) -> PathBuf {
    let mut path = base_storage_path(dev);

    // Construct the path to the ledger in storage.
    //
    // Prod: `~/.aleo/storage/bft-{network}/primary`
    // Dev: `path/to/repo/.bft-storage-{network}/primary-{id}`
    match dev {
        Some(id) => {
            path.push(format!(".bft-storage-{network}"));
            path.push(format!("primary-{id}"));
        }

        None => {
            path.push("storage");
            path.push(format!("bft-{network}"));
            path.push("primary");
        }
    }

    path
}

// Returns the path for the worker-related BFT files.
pub(crate) fn worker_storage_dir(network: u16, worker_id: u32, dev: Option<u16>) -> PathBuf {
    // Retrieve the starting directory.
    let mut path = base_storage_path(dev);

    // Construct the path to the ledger in storage.
    //
    // Prod: `~/.aleo/storage/bft-{network}/worker-{worker_id}`
    // Dev: `path/to/repo/.bft-storage-{network}/worker-{primary_id}-{worker_id}`
    match dev {
        Some(primary_id) => {
            path.push(format!(".bft-storage-{network}"));
            path.push(format!("worker-{primary_id}-{worker_id}"));
        }

        None => {
            path.push("storage");
            path.push(format!("bft-{network}"));
            path.push(format!("worker-{worker_id}"));
        }
    }

    path
}

// Returns the workspace path.
// TODO: move to a more appropriate place
pub fn workspace_dir() -> String {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().display().to_string()
}
