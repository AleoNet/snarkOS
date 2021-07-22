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

use tracing::*;

use super::*;

impl<S: KeyValueStorage + 'static> Agent<S> {
    pub(super) fn get_fork_path(&mut self, hash: &Digest, oldest_fork_threshold: usize) -> Result<ForkDescription> {
        let mut side_chain_path = VecDeque::new();
        let header = self.get_block_header(hash)?;
        let canon_height = self.canon_height()?;
        let mut parent_hash = header.previous_block_hash;
        for i in 0..=oldest_fork_threshold {
            // check if the part is part of the canon chain
            match self.get_block_state(&parent_hash)? {
                // This is a canon parent
                BlockStatus::Committed(block_num) => {
                    // Add the children from the latest block
                    if block_num + oldest_fork_threshold - i < canon_height as usize {
                        debug!("exceeded maximum fork length in extended path");
                        return Ok(ForkDescription::TooLong);
                    }
                    let longest_path = self.longest_child_path(hash)?;
                    side_chain_path.extend(longest_path);

                    return Ok(ForkDescription::Path(ForkPath {
                        base_index: block_num as u32,
                        path: side_chain_path.into(),
                    }));
                }
                // Add to the side_chain_path
                BlockStatus::Uncommitted => {
                    side_chain_path.push_front(parent_hash.clone());
                    parent_hash = self.get_block_header(&parent_hash)?.previous_block_hash;
                }
                BlockStatus::Unknown => {
                    return Ok(ForkDescription::Orphan);
                }
            }
        }
        Ok(ForkDescription::TooLong)
    }

    pub(super) fn commit_transaction(
        &mut self,
        sn_index: &mut u32,
        cm_index: &mut u32,
        memo_index: &mut u32,
        transaction: &SerialTransaction,
    ) -> Result<Vec<(Digest, u32)>> {
        let mut commitments = vec![];
        // we are leaving validation to the ledger
        for serial in transaction.old_serial_numbers.iter() {
            self.inner.store(
                KeyValueColumn::SerialNumber,
                &serial[..],
                &(*sn_index).to_le_bytes()[..],
            )?;
            *sn_index += 1;
        }

        for commitment in transaction.new_commitments.iter() {
            self.inner.store(
                KeyValueColumn::Commitment,
                &commitment[..],
                &(*cm_index).to_le_bytes()[..],
            )?;
            commitments.push((commitment.clone(), *cm_index));
            *cm_index += 1;
        }

        self.inner.store(
            KeyValueColumn::Memo,
            &transaction.memorandum[..],
            &(*memo_index).to_le_bytes()[..],
        )?;
        *memo_index += 1;

        Ok(commitments)
    }

    pub(super) fn commit_block(&mut self, block_hash: &Digest, ledger_digest: &Digest) -> Result<BlockStatus> {
        let canon = self.canon()?;
        let block = self.get_block(block_hash)?;
        match self.get_block_state(block_hash)? {
            BlockStatus::Committed(_) => return Err(StorageError::ExistingCanonBlock(hex::encode(block_hash)).into()),
            BlockStatus::Unknown => return Err(anyhow!("attempted to commit unknown block")),
            _ => (),
        }

        let mut sn_index = self.read_meta_u32(KEY_CURR_SN_INDEX, Some(0))?;
        let mut cm_index = self.read_meta_u32(KEY_CURR_CM_INDEX, Some(0))?;
        let mut memo_index = self.read_meta_u32(KEY_CURR_MEMO_INDEX, Some(0))?;

        let mut transaction_cms = vec![];

        for (index, transaction) in block.transactions.iter().enumerate() {
            let cms = self.commit_transaction(&mut sn_index, &mut cm_index, &mut memo_index, transaction)?;
            transaction_cms.extend(cms);

            let transaction_location = TransactionLocation {
                index: index as u32,
                block_hash: block_hash.clone(),
            };
            let mut out = vec![];
            transaction_location.write_le(&mut out)?;
            self.inner
                .store(KeyValueColumn::TransactionLookup, &transaction.id[..], &out[..])?;
        }

        self.write_meta_u32(KEY_CURR_SN_INDEX, sn_index)?;
        self.write_meta_u32(KEY_CURR_CM_INDEX, cm_index)?;
        self.write_meta_u32(KEY_CURR_MEMO_INDEX, memo_index)?;

        let is_genesis = canon.block_height == 0 && canon.hash.is_empty();

        let new_best_block_number = if is_genesis { 0 } else { canon.block_height as u32 + 1 };

        self.write_meta_u32(KEY_BEST_BLOCK_NUMBER, new_best_block_number)?;

        let block_num_serialized = &new_best_block_number.to_le_bytes()[..];

        self.inner
            .store(KeyValueColumn::BlockIndex, &block_hash[..], block_num_serialized)?;
        self.inner
            .store(KeyValueColumn::BlockIndex, block_num_serialized, &block_hash[..])?;

        self.inner
            .store(KeyValueColumn::DigestIndex, &ledger_digest[..], block_num_serialized)?;
        self.inner
            .store(KeyValueColumn::DigestIndex, block_num_serialized, &ledger_digest[..])?;

        Ok(BlockStatus::Committed(new_best_block_number as usize))
    }

    pub(super) fn decommit_transaction(
        &mut self,
        sn_index: &mut u32,
        cm_index: &mut u32,
        memo_index: &mut u32,
        transaction: &SerialTransaction,
    ) -> Result<()> {
        for serial in transaction.old_serial_numbers.iter() {
            self.inner.delete(KeyValueColumn::SerialNumber, &serial[..])?;
            *sn_index -= 1;
        }

        for commitment in transaction.new_commitments.iter() {
            self.inner.delete(KeyValueColumn::Commitment, &commitment[..])?;
            *cm_index -= 1;
        }

        self.inner
            .delete(KeyValueColumn::TransactionLookup, &transaction.id[..])?;

        self.inner.delete(KeyValueColumn::Memo, &transaction.memorandum[..])?;
        *memo_index -= 1;

        Ok(())
    }

    pub(super) fn decommit_blocks(&mut self, hash: &Digest) -> Result<Vec<SerialBlock>> {
        match self.get_block_state(hash)? {
            BlockStatus::Committed(_) => (),
            _ => return Err(anyhow!("attempted to decommit uncommitted block")),
        }
        let canon = self.canon()?;
        if canon.block_height == 0 {
            return Err(StorageError::InvalidBlockDecommit.into());
        }
        let mut canon_block_number = canon.block_height as u32;

        let mut sn_index = self.read_meta_u32(KEY_CURR_SN_INDEX, Some(0))?;
        let mut cm_index = self.read_meta_u32(KEY_CURR_CM_INDEX, Some(0))?;
        let mut memo_index = self.read_meta_u32(KEY_CURR_MEMO_INDEX, Some(0))?;

        let mut decommitted = vec![];

        let mut last_hash = canon.hash;
        loop {
            let block = self.get_block(&last_hash)?;
            let block_number = match self.get_block_state(&last_hash)? {
                BlockStatus::Unknown => return Err(anyhow!("unknown block state")),
                BlockStatus::Committed(n) => n as u32,
                BlockStatus::Uncommitted => return Err(anyhow!("uncommitted block in decommit")),
            };

            debug!("Decommitting block {} ({})", last_hash, block_number);
            for transaction in &block.transactions {
                debug!("Decommitting transaction {}", hex::encode(&transaction.id[..]));
                self.decommit_transaction(&mut sn_index, &mut cm_index, &mut memo_index, transaction)?;
            }

            let block_number_serialized = &block_number.to_le_bytes()[..];

            self.inner.delete(KeyValueColumn::BlockIndex, &last_hash[..])?;
            self.inner.delete(KeyValueColumn::BlockIndex, block_number_serialized)?;

            let digest = self
                .inner
                .get(KeyValueColumn::DigestIndex, block_number_serialized)?
                .ok_or_else(|| anyhow!("missing digest for block during decommiting"))?
                .into_owned();

            self.inner.delete(KeyValueColumn::DigestIndex, &digest)?;
            self.inner
                .delete(KeyValueColumn::DigestIndex, block_number_serialized)?;

            canon_block_number -= 1;
            let new_last_hash = block.header.previous_block_hash.clone();
            decommitted.push(block);
            if &last_hash == hash {
                break;
            }
            last_hash = new_last_hash;
        }

        self.write_meta_u32(KEY_CURR_SN_INDEX, sn_index)?;
        self.write_meta_u32(KEY_CURR_CM_INDEX, cm_index)?;
        self.write_meta_u32(KEY_CURR_MEMO_INDEX, memo_index)?;
        self.write_meta_u32(KEY_BEST_BLOCK_NUMBER, canon_block_number)?;

        Ok(decommitted)
    }

    pub(super) fn reset_ledger(
        &mut self,
        commitments: Vec<Digest>,
        serial_numbers: Vec<Digest>,
        memos: Vec<Digest>,
        digests: Vec<Digest>,
    ) -> Result<()> {
        let mut sn_index = 0u32;
        let mut cm_index = 0u32;
        let mut memo_index = 0u32;

        self.inner.truncate(KeyValueColumn::Commitment)?;
        self.inner.truncate(KeyValueColumn::SerialNumber)?;
        self.inner.truncate(KeyValueColumn::Memo)?;
        self.inner.truncate(KeyValueColumn::DigestIndex)?;

        for commitment in commitments.into_iter() {
            self.inner
                .store(KeyValueColumn::Commitment, &commitment[..], &cm_index.to_le_bytes()[..])?;
            cm_index += 1;
        }

        for serial_number in serial_numbers.into_iter() {
            self.inner.store(
                KeyValueColumn::SerialNumber,
                &serial_number[..],
                &sn_index.to_le_bytes()[..],
            )?;
            sn_index += 1;
        }

        for memo in memos.into_iter() {
            self.inner
                .store(KeyValueColumn::Memo, &memo[..], &memo_index.to_le_bytes()[..])?;
            memo_index += 1;
        }

        for (i, digest) in digests.into_iter().enumerate() {
            let block_num_serialized = &(i as u32).to_le_bytes()[..];
            self.inner
                .store(KeyValueColumn::DigestIndex, &digest[..], block_num_serialized)?;
            self.inner
                .store(KeyValueColumn::DigestIndex, block_num_serialized, &digest[..])?;
        }

        self.write_meta_u32(KEY_CURR_SN_INDEX, sn_index)?;
        self.write_meta_u32(KEY_CURR_CM_INDEX, cm_index)?;
        self.write_meta_u32(KEY_CURR_MEMO_INDEX, memo_index)?;
        Ok(())
    }
}
