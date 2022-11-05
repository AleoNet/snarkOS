// Copyright (C) 2019-2022 Aleo Systems Inc.
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

#![forbid(unsafe_code)]

#[macro_use]
extern crate tracing;

pub mod consensus;
pub use consensus::*;

mod memory_pool;

use snarkvm::prelude::*;

use anyhow::{anyhow, ensure, Result};
use colored::Colorize;
use core::time::Duration;
use futures::{Future, StreamExt};
use indexmap::IndexMap;
use parking_lot::RwLock;
use std::{net::IpAddr, sync::Arc};
use tokio::task;

type RecordMap<N> = IndexMap<Field<N>, Record<N, Plaintext<N>>>;

#[derive(Clone)]
pub struct Ledger<N: Network, C: ConsensusStorage<N>> {
    /// The consensus module.
    consensus: Arc<RwLock<Consensus<N, C>>>,
    /// The account private key.
    private_key: PrivateKey<N>,
    /// The account view key.
    view_key: ViewKey<N>,
    /// The account address.
    address: Address<N>,
}

impl<N: Network, C: ConsensusStorage<N>> Ledger<N, C> {
    /// Loads an instance of the ledger.
    pub fn load(private_key: PrivateKey<N>, genesis: Option<Block<N>>, dev: Option<u16>) -> Result<Self> {
        // Initialize consensus.
        let consensus = Arc::new(RwLock::new(Consensus::load(genesis, dev)?));
        // Return the ledger.
        Self::from(consensus, private_key)
    }

    /// Initializes a new instance of the ledger.
    pub fn from(consensus: Arc<RwLock<Consensus<N, C>>>, private_key: PrivateKey<N>) -> Result<Self> {
        // Derive the view key and address.
        let view_key = ViewKey::try_from(private_key)?;
        let address = Address::try_from(&view_key)?;

        // Return the ledger.
        Ok(Self { consensus, private_key, view_key, address })
    }

    /// Returns the consensus module.
    pub const fn consensus(&self) -> &Arc<RwLock<Consensus<N, C>>> {
        &self.consensus
    }

    /// Returns the ledger address.
    pub const fn address(&self) -> Address<N> {
        self.address
    }

    /// Adds the given unconfirmed transaction to the memory pool.
    pub fn add_unconfirmed_transaction(&self, transaction: Transaction<N>) -> Result<()> {
        self.consensus.write().add_unconfirmed_transaction(transaction)
    }

    /// Adds the given unconfirmed solution to the memory pool.
    pub fn add_unconfirmed_solution(&self, solution: &ProverSolution<N>) -> Result<()> {
        self.consensus.write().add_unconfirmed_solution(solution)
    }

    /// Returns the unspent records.
    pub fn find_unspent_records(&self) -> Result<RecordMap<N>> {
        Ok(self
            .consensus
            .read()
            .find_records(&self.view_key, RecordsFilter::Unspent)?
            .filter(|(_, record)| !record.gates().is_zero())
            .collect::<IndexMap<_, _>>())
    }

    /// Returns the spent records.
    pub fn find_spent_records(&self) -> Result<RecordMap<N>> {
        Ok(self
            .consensus
            .read()
            .find_records(&self.view_key, RecordsFilter::Spent)?
            .filter(|(_, record)| !record.gates().is_zero())
            .collect::<IndexMap<_, _>>())
    }

    /// Creates a deploy transaction.
    pub fn create_deploy(&self, program: &Program<N>, additional_fee: u64) -> Result<Transaction<N>> {
        // Fetch the unspent records.
        let records = self.find_unspent_records()?;
        ensure!(!records.len().is_zero(), "The Aleo account has no records to spend.");

        // Prepare the additional fee.
        let credits = records.values().max_by(|a, b| (**a.gates()).cmp(&**b.gates())).unwrap().clone();
        ensure!(***credits.gates() >= additional_fee, "The additional fee is more than the record balance.");

        // Initialize an RNG.
        let rng = &mut ::rand::thread_rng();
        // Deploy.
        let transaction = Transaction::deploy(
            self.consensus.read().vm(),
            &self.private_key,
            program,
            (credits, additional_fee),
            rng,
        )?;
        // Verify.
        assert!(self.consensus.read().vm().verify(&transaction));
        // Return the transaction.
        Ok(transaction)
    }

    /// Creates a transfer transaction.
    pub fn create_transfer(&self, to: &Address<N>, amount: u64) -> Result<Transaction<N>> {
        // Fetch the unspent records.
        let records = self.find_unspent_records()?;
        ensure!(!records.len().is_zero(), "The Aleo account has no records to spend.");

        // Initialize an RNG.
        let rng = &mut ::rand::thread_rng();

        // Create a new transaction.
        Transaction::execute(
            self.consensus.read().vm(),
            &self.private_key,
            &ProgramID::from_str("credits.aleo")?,
            Identifier::from_str("transfer")?,
            &[
                Value::Record(records.values().next().unwrap().clone()),
                Value::from_str(&format!("{to}"))?,
                Value::from_str(&format!("{amount}u64"))?,
            ],
            None,
            rng,
        )
    }
}
