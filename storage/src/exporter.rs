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

use anyhow::*;
use snarkvm_utilities::ToBytes;
use tracing::*;

use std::{fs, io::BufWriter, path::Path};

use crate::DynStorage;

/// Serializes the node's stored canon blocks into a single file written to `location`; `limit` specifies the limit
/// on the number of blocks to export, with `0` being no limit (a full export). Returns the number of exported
/// blocks.
pub async fn export_canon_blocks(storage: DynStorage, limit: u32, location: &Path) -> Result<usize, anyhow::Error> {
    info!("Exporting the node's canon blocks to {}", location.display());

    let blocks = storage.get_canon_blocks(Some(limit)).await?;

    let mut target_file = BufWriter::new(fs::File::create(location)?);
    for block in &blocks {
        block.write_le(&mut target_file)?;
    }

    Ok(blocks.len())
}
