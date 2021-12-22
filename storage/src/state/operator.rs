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

use crate::storage::{DataMap, Map, MapId, Storage};
use snarkvm::dpc::prelude::*;

use anyhow::{anyhow, Result};
use std::{collections::HashMap, path::Path};

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
    pub fn to_shares(&self) -> Vec<(u32, HashMap<Address<N>, u64>)> {
        self.shares.to_shares()
    }

    /// Returns the number of shares for a given block height.
    pub fn get_shares(&self, block_height: u32) -> Result<HashMap<Address<N>, u64>> {
        self.shares.get_shares(block_height)
    }

    /// Adds the given `num_shares` for an address in storage.
    pub fn add_shares(&self, block_height: u32, address: &Address<N>, num_shares: u64) -> Result<()> {
        self.shares.add_shares(block_height, address, num_shares)
    }

    /// Removes the shares for a given block height in storage.
    pub fn remove_shares(&self, block_height: u32) -> Result<()> {
        self.shares.remove_shares(block_height)
    }

    /// Returns `true` if the given commitment exists in storage.
    pub fn contains_coinbase_record(&self, commitment: &N::Commitment) -> Result<bool> {
        self.shares.contains_record(commitment)
    }

    /// Returns all coinbase records in storage.
    pub fn to_coinbase_records(&self) -> Vec<(u32, Record<N>)> {
        self.shares.to_records()
    }

    /// Returns the coinbase record for a given commitment.
    pub fn get_coinbase_record(&self, commitment: &N::Commitment) -> Result<(u32, Record<N>)> {
        self.shares.get_record(commitment)
    }

    /// Adds the given coinbase record to storage.
    pub fn add_coinbase_record(&self, block_height: u32, record: Record<N>) -> Result<()> {
        self.shares.add_record(block_height, record)
    }

    /// Removes the given record from storage.
    pub fn remove_coinbase_record(&self, commitment: &N::Commitment) -> Result<()> {
        self.shares.remove_record(commitment)
    }
}

#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
struct SharesState<N: Network> {
    /// The miner shares for each block height.
    shares: DataMap<u32, HashMap<Address<N>, u64>>,
    /// The coinbase records earned by the operator.
    records: DataMap<N::Commitment, (u32, Record<N>)>,
}

impl<N: Network> SharesState<N> {
    /// Initializes a new instance of `SharesState`.
    fn open<S: Storage>(storage: S) -> Result<Self> {
        Ok(Self {
            shares: storage.open_map(MapId::Shares)?,
            records: storage.open_map(MapId::PoolRecords)?,
        })
    }

    /// Returns all shares in storage.
    fn to_shares(&self) -> Vec<(u32, HashMap<Address<N>, u64>)> {
        self.shares.iter().collect()
    }

    /// Returns the shares for a given block height.
    fn get_shares(&self, block_height: u32) -> Result<HashMap<Address<N>, u64>> {
        match self.shares.get(&block_height)? {
            Some(shares) => Ok(shares),
            None => return Err(anyhow!("Block height {} does not have any shares in storage", block_height)),
        }
    }

    /// Adds the given number of shares to the block height and address in storage.
    fn add_shares(&self, block_height: u32, address: &Address<N>, num_shares: u64) -> Result<()> {
        if let Some(current_shares) = self.shares.get(&block_height)? {
            let mut new_shares = current_shares.clone();

            // Add the num shares for the address.
            let address_entry = new_shares.entry(*address).or_insert(0);
            *address_entry = address_entry.saturating_add(num_shares);

            // Insert the shares for the address.
            self.shares.insert(&block_height, &new_shares)?;
            Ok(())
        } else {
            // Insert the shares for the address.
            let mut new_shares = HashMap::new();
            new_shares.insert(*address, num_shares);
            self.shares.insert(&block_height, &new_shares)?;
            Ok(())
        }
    }

    /// Removes all of the shares for a given block height.
    fn remove_shares(&self, block_height: u32) -> Result<()> {
        self.shares.remove(&block_height)
    }

    /// Returns `true` if the given commitment exists in storage.
    fn contains_record(&self, commitment: &N::Commitment) -> Result<bool> {
        self.records.contains_key(commitment)
    }

    /// Returns all records in storage.
    fn to_records(&self) -> Vec<(u32, Record<N>)> {
        self.records.values().collect()
    }

    /// Returns the record for a given commitment.
    fn get_record(&self, commitment: &N::Commitment) -> Result<(u32, Record<N>)> {
        match self.records.get(commitment)? {
            Some((block_height, record)) => Ok((block_height, record)),
            None => return Err(anyhow!("Record with commitment {} does not exist in storage", commitment)),
        }
    }

    /// Adds the given block height and record to storage.
    fn add_record(&self, block_height: u32, record: Record<N>) -> Result<()> {
        // Ensure the record does not exist.
        let commitment = record.commitment();
        if self.records.contains_key(&commitment)? {
            Err(anyhow!("Record with commitment {} already exists in storage", commitment))
        } else {
            // Insert the record.
            self.records.insert(&commitment, &(block_height, record))?;
            Ok(())
        }
    }

    /// Removes the given record from storage.
    fn remove_record(&self, commitment: &N::Commitment) -> Result<()> {
        // Remove the record entry.
        self.records.remove(commitment)?;
        Ok(())
    }
}
