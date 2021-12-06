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
use std::path::Path;

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

    /// Returns `true` if the given address exists in storage.
    pub fn contains_address(&self, address: &Address<N>) -> Result<bool> {
        self.shares.contains_address(address)
    }

    /// Returns all the shares in storage.
    pub fn to_shares(&self) -> Vec<(Address<N>, u128)> {
        self.shares.to_shares()
    }

    /// Returns the number of shares for a given address.
    pub fn get_shares(&self, address: &Address<N>) -> Result<u128> {
        self.shares.get_shares(address)
    }

    /// Adds the given `num_shares` for an address in storage.
    pub fn add_shares(&self, block_height: u32, address: &Address<N>, num_shares: u128) -> Result<()> {
        self.shares.add_shares(block_height, address, num_shares)
    }

    /// Removes the given `num_shares` for an address in storage.
    pub fn remove_shares(&self, address: &Address<N>, num_shares: u128) -> Result<()> {
        self.shares.remove_shares(address, num_shares)
    }
}

#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
struct SharesState<N: Network> {
    // TODO (raychu86): Introduce concept of `rounds`.
    /// The miner shares.
    shares: DataMap<Address<N>, u128>,
}

impl<N: Network> SharesState<N> {
    /// Initializes a new instance of `SharesState`.
    fn open<S: Storage>(storage: S) -> Result<Self> {
        Ok(Self {
            shares: storage.open_map("shares")?,
        })
    }

    /// Returns `true` if the given address exists in storage.
    fn contains_address(&self, address: &Address<N>) -> Result<bool> {
        self.shares.contains_key(address)
    }

    /// Returns all shares in storage.
    fn to_shares(&self) -> Vec<(Address<N>, u128)> {
        self.shares.iter().collect()
    }

    /// Returns the record for a given address.
    fn get_shares(&self, address: &Address<N>) -> Result<u128> {
        match self.shares.get(address)? {
            Some(num_shares) => Ok(num_shares),
            None => return Err(anyhow!("Address {} does not have any shares in storage", address)),
        }
    }

    /// Adds the given number of shares to the address in storage.
    fn add_shares(&self, _block_height: u32, address: &Address<N>, num_shares: u128) -> Result<()> {
        // TODO (raychu86): Use block height to determine the round number.
        if let Some(current_num_shares) = self.shares.get(&address)? {
            let new_num_shares = current_num_shares.saturating_add(num_shares);
            // Insert the shares for the address.
            self.shares.insert(&address, &new_num_shares)?;
            Ok(())
        } else {
            // Insert the shares for the address.
            self.shares.insert(&address, &num_shares)?;
            Ok(())
        }
    }

    /// Removes the given number of shares for an address in storage.
    fn remove_shares(&self, address: &Address<N>, num_shares: u128) -> Result<()> {
        if let Some(current_num_shares) = self.shares.get(&address)? {
            match current_num_shares.saturating_sub(num_shares) {
                0 => self.shares.remove(&address)?,
                new_num_shares => self.shares.insert(&address, &new_num_shares)?,
            }
        }

        Ok(())
    }
}
