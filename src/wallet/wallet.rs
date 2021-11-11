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
use snarkvm::prelude::{FromBytes, ToBytes};

use anyhow::Result;
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
};

/// A wallet, belonging to a specific address. Currently keeps records of all
/// coinbases ever mined to this wallet, regardless of whether they are valid
/// or not on the current chain.
///
/// The path leading to the file containing all records will be structured as
/// {data_path}/{address}/data.txt.
pub struct Wallet<N: Network> {
    /// The address associated with this wallet.
    address: String,
    /// A path to a directory containing all coinbase records.
    data_path: String,
    _phantom: PhantomData<N>,
}

impl<N: Network> Wallet<N> {
    /// Creates a new [`Wallet`], and initializes a text file in the resulting
    /// path, if it doesn't yet exist.
    pub fn new(address: String, data_path: String) -> Result<Self> {
        let wallet = Self {
            address,
            data_path,
            _phantom: PhantomData,
        };
        if !wallet.path().exists() {
            fs::File::create(wallet.path())?;
        }

        Ok(wallet)
    }

    pub fn path_as_string(&self) -> String {
        format!("{}/{}/data.txt", self.data_path, self.address)
    }

    pub fn path(&self) -> PathBuf {
        Path::new(&self.path_as_string()).into()
    }

    /// Fetch all records from the database.
    pub fn records(&self) -> Result<Vec<Transaction<N>>> {
        let f = fs::File::open(self.path())?;
        let lines = BufReader::new(f).lines();
        let mut records = vec![];

        for (i, line) in lines.enumerate() {
            if let Ok(transaction_data) = line {
                let record = Transaction::<N>::read_le(transaction_data.as_bytes())?;
                records.push(record);
            } else {
                error!("Wallet file contains malformed transaction record at line {}", i);
            }
        }

        Ok(records)
    }

    /// Push a record to the database.
    pub fn push_record(&self, t: Transaction<N>) -> Result<()> {
        let mut f = OpenOptions::new().write(true).open(self.path())?;
        t.write_le(&mut f)?;
        f.write(b"\n")?;
        Ok(())
    }
}
