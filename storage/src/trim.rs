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

use crate::DynStorage;

use anyhow::*;
use tracing::*;

#[derive(Debug, Default)]
struct StorageTrimSummary {
    all_ops: usize,
    obsolete_blocks: usize,
    obsolete_txs: usize,
    updated_parents: usize,
}

/// Removes obsolete objects from the database; can be used for cleanup purposes, but it can also provide
/// some insight into the features of the chain, e.g. the number of blocks and transactions that were
/// ultimately not accepted into the canonical chain.
pub async fn trim(storage: DynStorage) -> Result<()> {
    info!("Checking for obsolete objects in the storage...");

    let non_canon_hashes = storage.get_block_hashes(None, crate::BlockFilter::NonCanonOnly).await?;

    info!("found {} obsolete blocks, removing...", non_canon_hashes.len());

    for hash in &non_canon_hashes {
        storage.delete_block(hash).await?;
    }

    info!(
        "The storage was trimmed successfully ({} items removed)!",
        non_canon_hashes.len()
    );

    Ok(())
}
