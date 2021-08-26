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

pub async fn migrate(from: &DynStorage, to: &DynStorage) -> Result<()> {
    let blocks = from.get_canon_blocks(None).await?;
    let ledger_digests = from.get_ledger_digests().await?;
    if blocks.len() != ledger_digests.len() {
        return Err(anyhow!(
            "canon, ledger digest lengths differed for migration -- corrupt DB?"
        ));
    }

    // transfer blocks
    for (block, digest) in blocks.into_iter().zip(ledger_digests.into_iter()) {
        to.insert_block(&block).await?;
        to.commit_block(&block.header.hash(), digest).await?;
    }

    // transfer miner records
    let record_commitments = from.get_record_commitments(None).await?;
    let mut records = Vec::with_capacity(record_commitments.len());
    for commitment in record_commitments {
        records.push(
            from.get_record(commitment)
                .await?
                .ok_or_else(|| anyhow!("missing record for commitment"))?,
        );
    }

    to.store_records(&records[..]).await?;

    Ok(())
}
