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

use crate::{
    state::LedgerState,
    storage::{DataMap, Map, MapId, Storage},
};
use snarkvm::dpc::prelude::*;

use anyhow::{anyhow, Result};
use std::{
    collections::{HashMap, HashSet},
    iter::FromIterator,
    path::Path,
    sync::Arc,
};

#[derive(Debug)]
pub struct OperatorState<N: Network> {
    shares: SharesState<N>,
}

impl<N: Network> OperatorState<N> {
    ///
    /// Opens a new writable instance of `OperatorState` from the given storage path.
    ///
    pub fn open_writer<S: Storage, P: AsRef<Path>>(path: P) -> Result<Self> {
        // Open storage.
        let context = N::NETWORK_ID;
        let is_read_only = false;
        let storage = S::open(path, context, is_read_only)?;

        // Initialize the operator.
        let operator = Self {
            shares: SharesState::open(storage)?,
        };

        info!("Operator successfully initialized");
        Ok(operator)
    }

    /// Returns all the shares in storage.
    pub fn to_shares(&self) -> Vec<((u32, Record<N>), HashMap<Address<N>, u64>)> {
        self.shares.to_shares()
    }

    /// Returns all coinbase records in storage.
    pub fn to_coinbase_records(&self) -> Vec<(u32, Record<N>)> {
        self.shares.to_records()
    }

    /// Returns the shares for a specific block, given the block height and coinbase record.
    pub fn get_shares_for_block(&self, block_height: u32, coinbase_record: Record<N>) -> Result<HashMap<Address<N>, u64>> {
        self.shares.get_shares_for_block(block_height, coinbase_record)
    }

    /// Returns the shares for a specific prover, given a ledger and the prover address.
    pub fn get_shares_for_prover(&self, ledger: &Arc<LedgerState<N>>, prover: &Address<N>) -> u64 {
        self.shares.get_shares_for_prover(ledger, prover)
    }

    /// Increments the share count by one for a given block height, coinbase record and prover address.
    pub fn increment_share(&self, block_height: u32, coinbase_record: Record<N>, prover: &Address<N>) -> Result<()> {
        self.shares.increment_share(block_height, coinbase_record, prover)
    }

    /// Removes the shares for a given block height and coinbase record in storage.
    pub fn remove_shares(&self, block_height: u32, coinbase_record: Record<N>) -> Result<()> {
        self.shares.remove_shares(block_height, coinbase_record)
    }

    /// Returns a list of provers which have submitted shares to an operator.
    pub fn get_provers(&self) -> Vec<Address<N>> {
        self.shares.get_provers()
    }
}

#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
struct SharesState<N: Network> {
    /// The miner shares for each block.
    shares: DataMap<(u32, Record<N>), HashMap<Address<N>, u64>>,
}

impl<N: Network> SharesState<N> {
    /// Initializes a new instance of `SharesState`.
    fn open<S: Storage>(storage: S) -> Result<Self> {
        Ok(Self {
            shares: storage.open_map(MapId::Shares)?,
        })
    }

    /// Returns all shares in storage.
    fn to_shares(&self) -> Vec<((u32, Record<N>), HashMap<Address<N>, u64>)> {
        self.shares.iter().collect()
    }

    /// Returns all records in storage.
    fn to_records(&self) -> Vec<(u32, Record<N>)> {
        self.shares.keys().collect()
    }

    /// Returns the shares for a specific block, given the block height and coinbase record.
    fn get_shares_for_block(&self, block_height: u32, coinbase_record: Record<N>) -> Result<HashMap<Address<N>, u64>> {
        match self.shares.get(&(block_height, coinbase_record))? {
            Some(shares) => Ok(shares),
            None => return Err(anyhow!("Block {} does not exist in shares storage", block_height)),
        }
    }

    /// Returns the shares for a specific prover, given a ledger and the prover address.
    fn get_shares_for_prover(&self, ledger: &Arc<LedgerState<N>>, prover: &Address<N>) -> u64 {
        self.shares
            .iter()
            .filter_map(|((_, coinbase_record), shares)| {
                if !shares.contains_key(prover) {
                    None
                } else {
                    match ledger.contains_commitment(&coinbase_record.commitment()) {
                        Ok(true) => shares.get(prover).copied(),
                        Ok(false) | Err(_) => None,
                    }
                }
            })
            .sum()
    }

    /// Increments the share count by one for a given block height, coinbase record, and prover address.
    fn increment_share(&self, block_height: u32, coinbase_record: Record<N>, prover: &Address<N>) -> Result<()> {
        // Retrieve the current shares for a given block height.
        let mut shares = match self.shares.get(&(block_height, coinbase_record.clone()))? {
            Some(shares) => shares,
            None => HashMap::new(),
        };

        // Increment the share count for the given address.
        let entry = shares.entry(*prover).or_insert(0);
        *entry = entry.saturating_add(1);

        // Insert the updated shares for the given block height.
        self.shares.insert(&(block_height, coinbase_record), &shares)
    }

    /// Removes all of the shares for a given block height and coinbase record.
    fn remove_shares(&self, block_height: u32, coinbase_record: Record<N>) -> Result<()> {
        self.shares.remove(&(block_height, coinbase_record))
    }

    fn get_provers(&self) -> Vec<Address<N>> {
        let set: HashSet<Address<N>> = self
            .shares
            .iter()
            .flat_map(|((_, _), shares)| shares.keys().copied().collect::<Vec<_>>())
            .collect();
        Vec::from_iter(set)
    }
}
