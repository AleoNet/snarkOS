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

use snarkvm::dpc::prelude::*;

use anyhow::Result;
use snarkos_ledger::storage::{rocksdb::RocksDB, DataMap, Map, Storage};
use std::path::PathBuf;

/// A wallet, belonging to a specific address. Currently keeps records of all
/// coinbases ever mined to this wallet, regardless of whether they are valid
/// or not on the current chain.
///
/// The path leading to the database containing all records will be structured as
/// {data_path}/{address}/.
///
/// The database will just store coinbase transactions under the block hash that
/// they belong to.
#[derive(Clone, Debug)]
pub struct Wallet<N: Network> {
    /// The address associated with this wallet.
    address: String,
    /// A path to a directory containing all coinbase records.
    data_path: String,
    /// The RocksDB containing all records for the given address.
    db: DataMap<N::BlockHash, Transaction<N>>,
}

impl<N: Network> Wallet<N> {
    /// Creates a new [`Wallet`].
    pub fn new(address: &str, data_path: &str, read_only: bool) -> Result<Self> {
        let path = PathBuf::from(format!("{}/{}/", data_path, address));

        Ok(Self {
            address: address.to_string(),
            data_path: data_path.to_string(),
            db: RocksDB::open(path, 0, read_only)?.open_map(&address)?,
        })
    }

    /// Fetch all records from the database.
    pub fn records(&self) -> Result<Vec<Transaction<N>>> {
        Ok(self.db.values().collect())
    }

    /// Push a record to the database.
    pub fn push_record(&self, hash: &N::BlockHash, tx: &Transaction<N>) -> Result<()> {
        self.db.insert(hash, tx)
    }
}
