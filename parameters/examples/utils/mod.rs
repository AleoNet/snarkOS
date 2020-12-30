// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use snarkvm_algorithms::crh::sha256::sha256;

use std::{
    fs::{self, File},
    io::{BufWriter, Result as IoResult, Write},
    path::PathBuf,
};

pub fn store(file_path: &PathBuf, checksum_path: &PathBuf, bytes: &[u8]) -> IoResult<()> {
    // Save checksum to file
    fs::write(checksum_path, hex::encode(sha256(bytes)))?;

    // Save buffer to file
    let mut file = BufWriter::new(File::create(file_path)?);
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}
