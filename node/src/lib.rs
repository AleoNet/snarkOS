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

pub use snarkos_node_bft as bft;
pub use snarkos_node_cdn as cdn;
pub use snarkos_node_consensus as consensus;
pub use snarkos_node_rest as rest;
pub use snarkos_node_router as router;
pub use snarkos_node_sync as sync;
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
        let mut path = aleo_std::aleo_ledger_dir(network, aleo_std::StorageMode::from(dev));

        // Delete the parent folder.
        path.pop();

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
            println!("{}.\n\n\nMIGRATION SUCCEEDED. RESTART THIS SNARKOS NODE AGAIN.\n\n", remove_ledger(N::ID, dev)?);
            // Sleep for 5 seconds to allow the user to read the message.
            std::thread::sleep(std::time::Duration::from_secs(5));
            return Ledger::<N, C>::load(genesis.clone(), dev.into());
        }
    } else if let Ok(block) = ledger.get_block(28251) {
        if *block.hash() == *ID::<N>::from_str("ab1ngmc9wf3kz73lxg9ylx75vday82a26xqthjykzrwyhngnr25uvqqau9eyh")? {
            let genesis = ledger.get_block(0)?;
            drop(ledger);
            println!("{}.\n\n\nMIGRATION SUCCEEDED. RESTART THIS SNARKOS NODE AGAIN.\n\n", remove_ledger(N::ID, dev)?);
            // Sleep for 5 seconds to allow the user to read the message.
            std::thread::sleep(std::time::Duration::from_secs(5));
            return Ledger::<N, C>::load(genesis.clone(), dev.into());
        }
    } else if let Ok(block) = ledger.get_block(28252) {
        if *block.hash() == *ID::<N>::from_str("ab1k6msq00mzrlmm3e0xzgynks5mqh2zrhd35akqqts24sd9u5x9yxs355qgv")? {
            let genesis = ledger.get_block(0)?;
            drop(ledger);
            println!("{}.\n\n\nMIGRATION SUCCEEDED. RESTART THIS SNARKOS NODE AGAIN.\n\n", remove_ledger(N::ID, dev)?);
            // Sleep for 5 seconds to allow the user to read the message.
            std::thread::sleep(std::time::Duration::from_secs(5));
            return Ledger::<N, C>::load(genesis.clone(), dev.into());
        }
    } else if let Ok(block) = ledger.get_block(115314) {
        if *block.hash() == *ID::<N>::from_str("ab13eckyhvhpv5zdhw8xz2zskrmm0a5hgeq7f5sjaw4errx0678pgpsjhuaqf")? {
            let genesis = ledger.get_block(0)?;
            drop(ledger);
            println!("{}.\n\n\nMIGRATION SUCCEEDED. RESTART THIS SNARKOS NODE AGAIN.\n\n", remove_ledger(N::ID, dev)?);
            // Sleep for 5 seconds to allow the user to read the message.
            std::thread::sleep(std::time::Duration::from_secs(5));
            return Ledger::<N, C>::load(genesis.clone(), dev.into());
        }
    } else if let Ok(block) = ledger.get_block(115315) {
        if *block.hash() == *ID::<N>::from_str("ab1axs5ltm6kjezsjxw35taf3xjpherrhpu6868h3ezhc3ap8pyrggqrrkjcg")? {
            let genesis = ledger.get_block(0)?;
            drop(ledger);
            println!("{}.\n\n\nMIGRATION SUCCEEDED. RESTART THIS SNARKOS NODE AGAIN.\n\n", remove_ledger(N::ID, dev)?);
            // Sleep for 5 seconds to allow the user to read the message.
            std::thread::sleep(std::time::Duration::from_secs(5));
            return Ledger::<N, C>::load(genesis.clone(), dev.into());
        }
    } else if let Ok(block) = ledger.get_block(726845) {
        if *block.hash() == *ID::<N>::from_str("ab1tf3v9qef0uh3ygsc0qqem7dzeyy2m8aqz583a80z60l8t5l22u9s84y38z")? {
            let genesis = ledger.get_block(0)?;
            drop(ledger);
            println!("{}.\n\n\nMIGRATION SUCCEEDED. RESTART THIS SNARKOS NODE AGAIN.\n\n", remove_ledger(N::ID, dev)?);
            // Sleep for 5 seconds to allow the user to read the message.
            std::thread::sleep(std::time::Duration::from_secs(5));
            return Ledger::<N, C>::load(genesis.clone(), dev.into());
        }
    } else if let Ok(block) = ledger.get_block(997810) {
        if *block.hash() == *ID::<N>::from_str("ab1pap9sxh5fcskw7l3msax4fq2mrqd80kxp0epx9dguxua2e8dacys78key5")? {
            let genesis = ledger.get_block(0)?;
            drop(ledger);
            println!("{}.\n\n\nMIGRATION SUCCEEDED. RESTART THIS SNARKOS NODE AGAIN.\n\n", remove_ledger(N::ID, dev)?);
            // Sleep for 5 seconds to allow the user to read the message.
            std::thread::sleep(std::time::Duration::from_secs(5));
            return Ledger::<N, C>::load(genesis.clone(), dev.into());
        }
    } else if let Ok(block) = ledger.get_block(997810) {
        if *block.hash() == *ID::<N>::from_str("ab1fx4mpz0fdqx75djf3n9grsjkc229xfs8fzmjqsxkajtj8j8sdurqufgvyz")? {
            let genesis = ledger.get_block(0)?;
            drop(ledger);
            println!("{}.\n\n\nMIGRATION SUCCEEDED. RESTART THIS SNARKOS NODE AGAIN.\n\n", remove_ledger(N::ID, dev)?);
            // Sleep for 5 seconds to allow the user to read the message.
            std::thread::sleep(std::time::Duration::from_secs(5));
            return Ledger::<N, C>::load(genesis.clone(), dev.into());
        }
    }
    Ok(ledger)
}

/// Starts the notification message loop.
pub fn start_notification_message_loop() -> tokio::task::JoinHandle<()> {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(180));
    tokio::spawn(async move {
        loop {
            interval.tick().await;
            // TODO (howardwu): Swap this with the official message for Testnet 3 announcements.
            // info!("{}", notification_message());
        }
    })
}

/// Returns the notification message as a string.
pub fn notification_message() -> String {
    use colored::Colorize;

    let mut output = String::new();
    output += &r#"

 ==================================================================================================

                     🚧 Welcome to Aleo Testnet 3 Phase 3 - Calibration Period 🚧

 ==================================================================================================

     During the calibration period, the network will be running in limited capacity.

     This calibration period is to ensure validators are stable and ready for mainnet launch.
     During this period, the objective is to assess, adjust, and align validators' performance,
     stability, and interoperability under varying network conditions.

     Please expect several network resets. With each network reset, software updates will
     be performed to address potential bottlenecks, vulnerabilities, and/or inefficiencies, which
     will ensure optimal performance for the ecosystem of validators, provers, and developers.

 ==================================================================================================

    Duration:
    - Start Date: September 27, 2023
    - End Date: October 18, 2023 (subject to change)

    Participation:
    - Node operators are NOT REQUIRED to participate during this calibration period.

    Network Resets:
    - IMPORTANT: EXPECT MULTIPLE NETWORK RESETS.
    - If participating, BE PREPARED TO RESET YOUR NODE AT ANY TIME.
    - When a reset occurs, RUN THE FOLLOWING TO RESET YOUR NODE:
        - git checkout testnet3 && git pull
        - cargo install --path .
        - snarkos clean
        - snarkos start --nodisplay --client

    Communication:
    - Stay ONLINE and MONITOR our Discord and Twitter for community updates.

    Purpose:
    - This period is STRICTLY FOR NETWORK CALIBRATION.
    - This period is NOT INTENDED for general-purpose usage by developers and provers.

    Incentives:
    - There are NO INCENTIVES during this calibration period.

 ==================================================================================================
"#
    .white()
    .bold();

    output
}
