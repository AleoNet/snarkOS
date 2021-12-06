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

use crate::storage::{DataMap, Map, Storage};
use snarkvm::dpc::prelude::*;

use anyhow::{anyhow, Result};
use std::{collections::HashMap, path::Path};

#[derive(Debug)]
pub struct MiningPoolState<N: Network> {
    shares: SharesState<N>,
}

impl<N: Network> MiningPoolState<N> {
    ///
    /// Opens a new writable instance of `MiningPoolState` from the given storage path.
    ///
    pub fn open_writer<S: Storage, P: AsRef<Path>>(path: P) -> Result<Self> {
        // Open storage.
        let context = N::NETWORK_ID;
        let is_read_only = false;
        let storage = S::open(path, context, is_read_only)?;

        // Initialize the mining pool.
        let mining_pool = Self {
            shares: SharesState::open(storage)?,
        };

        info!("Mining pool successfully initialized");
        Ok(mining_pool)
    }

    /// Returns all the shares in storage.
    pub fn to_shares(&self) -> Vec<(u32, HashMap<Address<N>, u128>)> {
        self.shares.to_shares()
    }

    /// Returns the number of shares for a given block_height.
    pub fn get_shares(&self, block_height: u32) -> Result<HashMap<Address<N>, u128>> {
        self.shares.get_shares(block_height)
    }

    /// Adds the given `num_shares` for an address in storage.
    pub fn add_shares(&self, block_height: u32, address: &Address<N>, num_shares: u128) -> Result<()> {
        self.shares.add_shares(block_height, address, num_shares)
    }

    /// Removes the shares for a given block height in storage.
    pub fn remove_shares(&self, block_height: u32) -> Result<()> {
        self.shares.remove_shares(block_height)
    }
}

#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
struct SharesState<N: Network> {
    // TODO (raychu86): Introduce concept of `rounds`.
    /// The miner shares for each block height.
    shares: DataMap<u32, HashMap<Address<N>, u128>>,
}

impl<N: Network> SharesState<N> {
    /// Initializes a new instance of `SharesState`.
    fn open<S: Storage>(storage: S) -> Result<Self> {
        Ok(Self {
            shares: storage.open_map("shares")?,
        })
    }

    /// Returns all shares in storage.
    fn to_shares(&self) -> Vec<(u32, HashMap<Address<N>, u128>)> {
        self.shares.iter().collect()
    }

    /// Returns the shares for a given block height.
    fn get_shares(&self, block_height: u32) -> Result<HashMap<Address<N>, u128>> {
        match self.shares.get(&block_height)? {
            Some(shares) => Ok(shares),
            None => return Err(anyhow!("Block height {} does not have any shares in storage", block_height)),
        }
    }

    /// Adds the given number of shares to the block height and address in storage.
    fn add_shares(&self, block_height: u32, address: &Address<N>, num_shares: u128) -> Result<()> {
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
}
