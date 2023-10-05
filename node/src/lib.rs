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

#![forbid(unsafe_code)]
#![allow(clippy::too_many_arguments)]
#![recursion_limit = "256"]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate tracing;

pub use snarkos_node_cdn as cdn;
pub use snarkos_node_consensus as consensus;
pub use snarkos_node_narwhal as narwhal;
pub use snarkos_node_rest as rest;
pub use snarkos_node_router as router;
pub use snarkos_node_tcp as tcp;
pub use snarkvm;

mod client;
pub use client::*;

mod prover;
pub use prover::*;

mod validator;
pub use validator::*;

mod node;
pub use node::*;

mod traits;
pub use traits::*;

/// A helper to log instructions to recover.
pub fn log_clean_error(dev: Option<u16>) {
    match dev {
        Some(id) => error!("Storage corruption detected! Run `snarkos clean --dev {id}` to reset storage"),
        None => error!("Storage corruption detected! Run `snarkos clean` to reset storage"),
    }
}

use snarkvm::{
    ledger::store::ConsensusStorage,
    prelude::{const_assert, hrp2, AleoID, Field, Ledger, Network},
};

use anyhow::{bail, Result};

// TODO: Remove me after Phase 3.
pub fn phase_3_reset<N: Network, C: ConsensusStorage<N>>(
    ledger: Ledger<N, C>,
    dev: Option<u16>,
) -> Result<Ledger<N, C>> {
    use core::str::FromStr;

    /// Removes the specified ledger from storage.
    pub(crate) fn remove_ledger(network: u16, dev: Option<u16>) -> Result<String> {
        // Construct the path to the ledger in storage.
        let path = aleo_std::aleo_ledger_dir(network, dev);

        // Prepare the path string.
        let path_string = format!("(in \"{}\")", path.display());

        // Check if the path to the ledger exists in storage.
        if path.exists() {
            // Remove the ledger files from storage.
            match std::fs::remove_dir_all(&path) {
                Ok(_) => Ok(format!("✅ Cleaned the snarkOS node storage {path_string}")),
                Err(error) => {
                    bail!("Failed to remove the snarkOS node storage {path_string}\n{}", error.to_string())
                }
            }
        } else {
            Ok(format!("✅ No snarkOS node storage was found {path_string}"))
        }
    }

    type ID<N> = AleoID<Field<N>, { hrp2!("ab") }>;

    if let Ok(block) = ledger.get_block(28250) {
        if *block.hash() == *ID::<N>::from_str("ab1fxetqjm0ppruay8vlg6gtt52d5fkeydmrk0talp04ymjm65acg9sh8d0r5")? {
            let genesis = ledger.get_block(0)?;
            drop(ledger);
            println!("{}", remove_ledger(N::ID, dev)?);
            return Ledger::<N, C>::load(genesis.clone(), dev);
        }
    } else if let Ok(block) = ledger.get_block(28251) {
        if *block.hash() == *ID::<N>::from_str("ab1ngmc9wf3kz73lxg9ylx75vday82a26xqthjykzrwyhngnr25uvqqau9eyh")? {
            let genesis = ledger.get_block(0)?;
            drop(ledger);
            println!("{}", remove_ledger(N::ID, dev)?);
            return Ledger::<N, C>::load(genesis.clone(), dev);
        }
    } else if let Ok(block) = ledger.get_block(28252) {
        if *block.hash() == *ID::<N>::from_str("ab1k6msq00mzrlmm3e0xzgynks5mqh2zrhd35akqqts24sd9u5x9yxs355qgv")? {
            let genesis = ledger.get_block(0)?;
            drop(ledger);
            println!("{}", remove_ledger(N::ID, dev)?);
            return Ledger::<N, C>::load(genesis.clone(), dev);
        }
    }
    Ok(ledger)
}
