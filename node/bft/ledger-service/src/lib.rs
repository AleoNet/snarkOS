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

#[macro_use]
extern crate async_trait;

#[cfg(feature = "ledger")]
pub mod ledger;
#[cfg(feature = "ledger")]
pub use ledger::*;

#[cfg(feature = "mock")]
pub mod mock;
#[cfg(feature = "mock")]
pub use mock::*;

#[cfg(feature = "prover")]
pub mod prover;
#[cfg(feature = "prover")]
pub use prover::*;

#[cfg(feature = "translucent")]
pub mod translucent;
#[cfg(feature = "translucent")]
pub use translucent::*;

#[cfg(feature = "metrics")]
use rayon::iter::ParallelIterator;
#[cfg(feature = "metrics")]
use snarkvm::prelude::{Block, Network};
#[cfg(feature = "metrics")]
use std::sync::atomic::{AtomicUsize, Ordering};

pub mod traits;
pub use traits::*;

/// Formats an ID into a truncated identifier (for logging purposes).
pub fn fmt_id(id: impl ToString) -> String {
    let id = id.to_string();
    let mut formatted_id = id.chars().take(16).collect::<String>();
    if id.chars().count() > 16 {
        formatted_id.push_str("..");
    }
    formatted_id
}

/// A helper macro to spawn a blocking task.
#[macro_export]
macro_rules! spawn_blocking {
    ($expr:expr) => {
        match tokio::task::spawn_blocking(move || $expr).await {
            Ok(value) => value,
            Err(error) => Err(snarkvm::prelude::anyhow!("[tokio::spawn_blocking] {error}")),
        }
    };
}

#[cfg(feature = "metrics")]
fn update_block_metrics<N: Network>(block: &Block<N>) {
    use snarkvm::ledger::ConfirmedTransaction;

    let accepted_deploy = AtomicUsize::new(0);
    let accepted_execute = AtomicUsize::new(0);
    let rejected_deploy = AtomicUsize::new(0);
    let rejected_execute = AtomicUsize::new(0);

    // Add transaction to atomic counter based on enum type match.
    block.transactions().par_iter().for_each(|tx| match tx {
        ConfirmedTransaction::AcceptedDeploy(_, _, _) => {
            accepted_deploy.fetch_add(1, Ordering::Relaxed);
        }
        ConfirmedTransaction::AcceptedExecute(_, _, _) => {
            accepted_execute.fetch_add(1, Ordering::Relaxed);
        }
        ConfirmedTransaction::RejectedDeploy(_, _, _, _) => {
            rejected_deploy.fetch_add(1, Ordering::Relaxed);
        }
        ConfirmedTransaction::RejectedExecute(_, _, _, _) => {
            rejected_execute.fetch_add(1, Ordering::Relaxed);
        }
    });

    metrics::increment_gauge(metrics::blocks::ACCEPTED_DEPLOY, accepted_deploy.load(Ordering::Relaxed) as f64);
    metrics::increment_gauge(metrics::blocks::ACCEPTED_EXECUTE, accepted_execute.load(Ordering::Relaxed) as f64);
    metrics::increment_gauge(metrics::blocks::REJECTED_DEPLOY, rejected_deploy.load(Ordering::Relaxed) as f64);
    metrics::increment_gauge(metrics::blocks::REJECTED_EXECUTE, rejected_execute.load(Ordering::Relaxed) as f64);

    // Update aborted transactions and solutions.
    metrics::increment_gauge(metrics::blocks::ABORTED_TRANSACTIONS, block.aborted_transaction_ids().len() as f64);
    metrics::increment_gauge(metrics::blocks::ABORTED_SOLUTIONS, block.aborted_solution_ids().len() as f64);
}
