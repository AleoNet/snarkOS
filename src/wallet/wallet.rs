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

use anyhow::Result;
use std::fs;

/// A wallet, belonging to a specific address. Currently keeps records of all
/// coinbases ever mined to this wallet, regardless of whether they are valid
/// or not on the current chain.
///
/// The path leading to the file containing all records will be structured as
/// {data_path}/{address}/data.txt.
pub struct Wallet {
    /// The address associated with this wallet.
    address: String,
    /// A path to a directory containing all coinbase records.
    data_path: String,
}

impl Wallet {
    /// Creates a new [`Wallet`], and initializes a text file in the given
    /// `data_path`, if it doesn't yet exist.
    pub fn new(address: String, data_path: String) -> Result<Self> {
        let path = format!("{}/{}/data.txt", data_path, address);
        if !Path::new(path).exists() {
            fs::create(path)?;
        }

        Self { address, data_path }
    }
}
