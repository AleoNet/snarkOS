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

use std::sync::Arc;

use crate::{
    error::ConsensusError,
    Consensus,
    ConsensusParameters,
    CreatePartialTransactionRequest,
    DynLedger,
    MemoryPool,
};
use anyhow::*;
use snarkos_storage::{BlockFilter, BlockOrder, BlockStatus, Digest, DynStorage, ForkDescription, SerialBlock, SerialTransaction, VMTransaction};
use snarkvm_dpc::{
    testnet1::{instantiated::Components, Record as DPCRecord, TransactionKernel},
    DPCScheme,
};
use snarkvm_posw::txids_to_roots;
use tokio::sync::mpsc;

use snarkos_metrics::misc::*;

use rand::thread_rng;

use super::message::{ConsensusMessage, CreateTransactionRequest, TransactionResponse};

mod agent;
mod commit;
mod transaction;

pub struct ConsensusInner {
    pub public: Arc<Consensus>,
    pub ledger: DynLedger,
    pub memory_pool: MemoryPool,
    pub storage: DynStorage,
}

struct LedgerData {
    ledger: DynLedger,
    commitments: Vec<Digest>,
    serial_numbers: Vec<Digest>,
    memos: Vec<Digest>,
    ledger_digests: Vec<Digest>,
}

impl ConsensusInner {
    /// scans uncommitted blocks for forks
    async fn scan_forks(&mut self) -> Result<Vec<(Digest, Digest)>> {
        let canon_hashes = self
            .storage
            .get_block_hashes(
                Some(crate::OLDEST_FORK_THRESHOLD as u32 + 1),
                BlockFilter::CanonOnly(BlockOrder::Descending),
            )
            .await?;

        if canon_hashes.len() < 2 {
            // windows will panic if len < 2
            return Ok(vec![]);
        }

        let mut known_forks = vec![];

        for canon_hashes in canon_hashes.windows(2) {
            // windows will ignore last block (furthest down), so we pull one extra above
            let target_hash = &canon_hashes[1];
            let ignore_child_hash = &canon_hashes[0];
            let children = self.storage.get_block_children(target_hash).await?;
            if children.len() == 1 && &children[0] == ignore_child_hash {
                continue;
            }
            for child in children {
                if &child != ignore_child_hash {
                    known_forks.push((target_hash.clone(), child));
                }
            }
        }

        Ok(known_forks)
    }

    fn fresh_ledger(&self, blocks: Vec<SerialBlock>) -> Result<LedgerData> {
        let mut ledger = self.ledger.clone();
        ledger.clear();
        let mut new_commitments = vec![];
        let mut new_serial_numbers = vec![];
        let mut new_memos = vec![];
        let mut new_digests = vec![];
        for (i, block) in blocks.into_iter().enumerate() {
            trace!("ledger recreation: processing block {}", i);
            let mut commitments = vec![];
            let mut serial_numbers = vec![];
            let mut memos = vec![];
            for transaction in block.transactions.iter() {
                commitments.extend_from_slice(&transaction.new_commitments[..]);
                serial_numbers.extend_from_slice(&transaction.old_serial_numbers[..]);
                memos.push(transaction.memorandum.clone());
            }
            let digest = ledger.extend(&commitments[..], &serial_numbers[..], &memos[..])?;
            new_commitments.extend(commitments);
            new_serial_numbers.extend(serial_numbers);
            new_memos.extend(memos);
            new_digests.push(digest);
        }
        Ok(LedgerData {
            ledger,
            commitments: new_commitments,
            serial_numbers: new_serial_numbers,
            memos: new_memos,
            ledger_digests: new_digests,
        })
    }

    #[allow(dead_code)]
    /// diagnostic function for storage/consensus consistency issues
    async fn diff_canon(&self) -> Result<()> {
        let blocks = self.storage.get_canon_blocks(Some(128)).await?;
        info!("diffing canon for {} blocks", blocks.len());
        let data = self.fresh_ledger(blocks)?;
        let commitments = self.storage.get_commitments().await?;
        let serial_numbers = self.storage.get_serial_numbers().await?;
        let memos = self.storage.get_memos().await?;
        let ledger_digests = self.storage.get_ledger_digests().await?;

        fn diff(name: &str, calculated: &[Digest], stored: &[Digest]) {
            info!(
                "diffing {}: {} calculated vs {} stored",
                name,
                calculated.len(),
                stored.len()
            );
            let max_len = calculated.len().max(stored.len());
            for i in 0..max_len {
                if calculated.get(i) != stored.get(i) {
                    error!(
                        "diff {}: mismatch @ {}: {} calculated != {} stored",
                        name,
                        i,
                        calculated
                            .get(i)
                            .map(|x| format!("{}", x))
                            .unwrap_or_else(|| "missing".to_string()),
                        stored
                            .get(i)
                            .map(|x| format!("{}", x))
                            .unwrap_or_else(|| "missing".to_string())
                    );
                }
            }
        }
        diff("commitments", &data.commitments[..], &commitments[..]);
        diff("serial_numbers", &data.serial_numbers[..], &serial_numbers[..]);
        diff("memos", &data.memos[..], &memos[..]);
        diff("ledger_digests", &data.ledger_digests[..], &ledger_digests[..]);
        Ok(())
    }

    /// helper function to rebuild a broken ledger index in storage.
    /// used in debugging, preserved for future potential use
    async fn recommit_canon(&mut self) -> Result<()> {
        let blocks = self.storage.get_canon_blocks(None).await?;
        let block_count = blocks.len();
        info!("recommiting {} blocks", block_count);
        let data = self.fresh_ledger(blocks)?;
        self.ledger = data.ledger;
        info!("resetting ledger for {} blocks", block_count);
        self.storage
            .reset_ledger(data.commitments, data.serial_numbers, data.memos, data.ledger_digests)
            .await?;
        Ok(())
    }
}
