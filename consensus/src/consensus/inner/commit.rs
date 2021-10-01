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

use std::time::Instant;

use super::*;
use crate::ledger::dummy::DummyLedger;
use snarkos_metrics as metrics;
use snarkos_storage::DigestTree;
use snarkvm_dpc::AleoAmount;
use tokio::task;

impl ConsensusInner {
    /// Receive a block from an external source and process it based on ledger state.
    pub(super) async fn receive_block(&mut self, block: &SerialBlock) -> Result<(), ConsensusError> {
        let hash = block.header.hash();
        match self.storage.get_block_state(&hash).await? {
            BlockStatus::Unknown => (),
            BlockStatus::Committed(_) | BlockStatus::Uncommitted => {
                metrics::increment_counter!(metrics::blocks::DUPLICATES);
                return Err(ConsensusError::PreExistingBlock);
            }
        }
        self.storage.insert_block(block).await?;

        match self.try_commit_block(&hash, block).await {
            Err(ConsensusError::InvalidBlock(hash)) => {
                self.storage.delete_block(&hash).await?;
                return Err(ConsensusError::InvalidBlock(hash));
            }
            Ok(_) => {}
            err => return err,
        }

        self.try_to_fast_forward().await?;

        Ok(())
    }

    pub(super) async fn try_commit_block(&mut self, hash: &Digest, block: &SerialBlock) -> Result<(), ConsensusError> {
        let canon = self.storage.canon().await?;

        match self.storage.get_block_state(&block.header.previous_block_hash).await? {
            BlockStatus::Committed(n) if n == canon.block_height => {
                debug!("Processing a block that is on canon chain. Height {} -> {}", n, n + 1);
                metrics::gauge!(metrics::blocks::HEIGHT, n as f64 + 1.0);
                // Process the block now.
            }
            BlockStatus::Unknown => {
                debug!("Processing a block that is an unknown orphan");
                // Don't process the block.
                return Ok(());
            }
            _ => {
                let fork_path = self
                    .storage
                    .get_fork_path(&block.header.previous_block_hash, crate::OLDEST_FORK_THRESHOLD)
                    .await?;
                match fork_path {
                    ForkDescription::Path(fork_path) => {
                        let new_block_number = fork_path.base_index + fork_path.path.len() as u32;
                        debug!("Processing a block that is on side chain. Height {}", new_block_number);
                        // If the side chain is now longer than the canon chain,
                        // perform a fork to the side chain.
                        if new_block_number as usize > canon.block_height {
                            debug!(
                                "Determined side chain is longer than canon chain by {} blocks",
                                new_block_number as usize - canon.block_height
                            );
                            warn!("A valid fork has been detected. Performing a fork to the side chain.");

                            let head_header = self.storage.get_block_header(&fork_path.path[0]).await?;
                            let canon_branch_number =
                                match self.storage.get_block_state(&head_header.previous_block_hash).await? {
                                    BlockStatus::Unknown => {
                                        return Err(anyhow!("failed to find parent block of fork").into());
                                    }
                                    BlockStatus::Committed(n) => n,
                                    BlockStatus::Uncommitted => {
                                        return Err(anyhow!("proposed parent block of fork is non-canon").into());
                                    }
                                };

                            // Remove existing canon chain descendents, if any.
                            match self.storage.get_block_hash(canon_branch_number as u32 + 1).await? {
                                None => (),
                                Some(hash) => {
                                    self.decommit_ledger_block(&hash).await?;
                                }
                            };

                            {
                                let canon = self.storage.canon().await?;
                                metrics::gauge!(metrics::blocks::HEIGHT, canon.block_height as f64);
                            }

                            for block_hash in fork_path.path {
                                if &block_hash == hash {
                                    self.verify_and_commit_block(hash, block).await?
                                } else {
                                    let new_block = self.storage.get_block(&block_hash).await?;
                                    self.verify_and_commit_block(&new_block.header.hash(), &new_block)
                                        .await?;
                                }
                            }
                        } else {
                            // Don't process the block.
                            return Ok(());
                        }
                    }
                    ForkDescription::Orphan => {
                        debug!("Processing a block that is on unknown orphan chain");
                        // Don't process the block.
                        return Ok(());
                    }
                    ForkDescription::TooLong => {
                        debug!("Processing a block that is on an over-length fork");
                        // Don't process the block.
                        return Ok(());
                    }
                }
            }
        }

        // Process the block.
        self.verify_and_commit_block(hash, block).await?;

        Ok(())
    }

    /// Return whether or not the given block is valid and insert it.
    /// 1. Verify that the block header is valid.
    /// 2. Verify that the transactions are valid.
    /// 3. Insert/canonize block.
    pub(super) async fn verify_and_commit_block(
        &mut self,
        hash: &Digest,
        block: &SerialBlock,
    ) -> Result<(), ConsensusError> {
        let now = std::time::Instant::now();

        match self.storage.get_block_state(hash).await? {
            BlockStatus::Committed(_) => return Ok(()),
            BlockStatus::Unknown => return Err(ConsensusError::InvalidBlock(hash.clone())),
            BlockStatus::Uncommitted => (),
        }

        // 1. Verify that the block valid
        if !self.verify_block(block).await? {
            return Err(ConsensusError::InvalidBlock(hash.clone()));
        }

        // 2. Insert/canonize block
        self.commit_block(hash, block).await?;

        // 3. Remove transactions from the mempool
        for transaction in block.transactions.iter() {
            self.memory_pool.remove(&transaction.id.into())?;
        }

        metrics::histogram!(metrics::blocks::COMMIT_TIME, now.elapsed());

        Ok(())
    }

    /// Check if the block is valid.
    /// Verify transactions and transaction fees.
    pub(super) async fn verify_block(&mut self, block: &SerialBlock) -> Result<bool, ConsensusError> {
        let canon = self.storage.canon().await?;
        // Verify the block header
        if block.header.previous_block_hash != canon.hash {
            return Err(anyhow!("attempted to commit a block that wasn't a direct child of tip of canon").into());
        }

        // Verify block amounts and check that there is a single coinbase transaction
        let mut coinbase_transaction_count: i32 = 0;
        let mut total_value_balance = AleoAmount::ZERO;

        for transaction in block.transactions.iter() {
            let value_balance = transaction.value_balance;

            if value_balance.is_negative() {
                coinbase_transaction_count += 1;
                if coinbase_transaction_count > 1 {
                    error!("multiple coinbase transactions");
                    return Ok(false);
                }
            }

            total_value_balance = total_value_balance.add(value_balance);
        }
        if coinbase_transaction_count == 0 {
            error!("missing coinbase transaction");
            return Ok(false);
        }

        // Check that there is only 1 coinbase transaction
        // Check that the block value balances are correct
        let expected_block_reward = crate::get_block_reward(canon.block_height as u32).0;
        if total_value_balance.0 + expected_block_reward != 0 {
            trace!("total_value_balance: {:?}", total_value_balance);
            trace!("expected_block_reward: {:?}", expected_block_reward);

            return Ok(false);
        }

        let block_header = block.header.clone();
        let parent_header = self.storage.get_block_header(&canon.hash).await?;
        let transaction_ids: Vec<[u8; 32]> = block.transactions.iter().map(|x| x.id).collect();
        let consensus = self.public.clone();

        let verification_result = task::spawn_blocking(move || {
            let (merkle_root, pedersen_merkle_root, _) = txids_to_roots(&transaction_ids);

            if let Err(err) =
                consensus
                    .parameters
                    .verify_header(&block_header, &parent_header, &merkle_root, &pedersen_merkle_root)
            {
                error!("Block header failed to verify: {:?}", err);
                false
            } else {
                true
            }
        })
        .await?;

        if !verification_result {
            return Ok(false);
        }

        // Check that all the transaction proofs verify
        self.verify_transactions(block.transactions.clone()).await
    }

    pub(super) async fn commit_block(&mut self, hash: &Digest, block: &SerialBlock) -> Result<(), ConsensusError> {
        let mut commitments = vec![];
        let mut serial_numbers = vec![];
        let mut memos = vec![];
        for transaction in block.transactions.iter() {
            commitments.extend_from_slice(&transaction.new_commitments[..]);
            serial_numbers.extend_from_slice(&transaction.old_serial_numbers[..]);
            memos.push(transaction.memorandum.clone());
        }

        let digest = if self.ledger.requires_async_task(commitments.len(), serial_numbers.len()) {
            let mut ledger = std::mem::replace(&mut self.ledger, DynLedger(Box::new(DummyLedger)));
            let (digest, ledger) = tokio::task::spawn_blocking(move || {
                let digest = ledger.extend(&commitments[..], &serial_numbers[..], &memos[..]);

                (digest, ledger)
            })
            .await?;

            self.ledger = ledger;

            digest?
        } else {
            self.ledger.extend(&commitments[..], &serial_numbers[..], &memos[..])?
        };

        self.storage.commit_block(hash, digest).await?;
        self.cleanse_memory_pool()
    }

    pub(super) async fn try_to_fast_forward(&mut self) -> Result<(), ConsensusError> {
        let canon = self.storage.canon().await?;
        let mut children = self.storage.get_block_digest_tree(&canon.hash).await?;
        if matches!(&children, DigestTree::Leaf(x) if x == &canon.hash) {
            return Ok(());
        }
        debug!(
            "Attempting to canonize the descendants of block at height {}.",
            canon.block_height
        );
        loop {
            let mut sub_children = children.take_children();
            // rust doesn't believe we will always set children before the next loop
            children = DigestTree::Leaf(Digest::from([0u8; 32]));
            if sub_children.is_empty() {
                break;
            }
            sub_children.sort_by_key(|child| std::cmp::Reverse(child.longest_length()));
            debug!("Processing the next known descendant.");
            let mut last_error = None;
            for child in sub_children {
                let new_block = self.storage.get_block(child.root()).await?;
                match self.try_commit_block(child.root(), &new_block).await {
                    Ok(()) => {
                        children = child;
                        last_error = None;
                        break;
                    }
                    Err(e) => {
                        warn!("failed to commit descendent block, trying sibling... error: {:?}", e);
                        last_error = Some(e);
                    }
                }
            }
            if let Some(last_error) = last_error {
                return Err(last_error);
            }
        }
        Ok(())
    }

    /// removes a block and all of it's descendents from storage and ledger
    pub(super) async fn decommit_ledger_block(&mut self, hash: &Digest) -> Result<(), ConsensusError> {
        let decommited_blocks = self.storage.decommit_blocks(hash).await?;
        let mut commitments = vec![];
        let mut serial_numbers = vec![];
        let mut memos = vec![];
        debug!("decommited {} blocks", decommited_blocks.len());
        for block in decommited_blocks.into_iter().rev() {
            debug!("ledger: rolling back block {}", block.header.hash());
            for transaction in block.transactions.iter() {
                commitments.extend_from_slice(&transaction.new_commitments[..]);
                serial_numbers.extend_from_slice(&transaction.old_serial_numbers[..]);
                memos.push(transaction.memorandum.clone());
            }
        }

        self.ledger
            .rollback(&commitments[..], &serial_numbers[..], &memos[..])?;
        self.cleanse_memory_pool()
    }
}
