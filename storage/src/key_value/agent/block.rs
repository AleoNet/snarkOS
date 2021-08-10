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

use std::{collections::HashSet, convert::TryInto};

use tracing::trace;

use crate::BlockOrder;

use super::*;

impl<S: KeyValueStorage + Validator + 'static> Agent<S> {
    pub(super) fn get_child_block_hashes(&mut self, hash: &Digest) -> Result<Vec<Digest>> {
        Ok(self
            .inner()
            .get(KeyValueColumn::ChildHashes, &hash[..])?
            .map(|x| bincode::deserialize::<'_, Vec<BlockHeaderHash>>(&x[..]))
            .transpose()?
            .map(|x| x.into_iter().map(|x| x.0[..].into()).collect())
            .unwrap_or_else(Vec::new))
    }

    fn update_child_block_hashes(&mut self, hash: &Digest, new_children: &[Digest]) -> Result<()> {
        if new_children.is_empty() {
            self.inner().delete(KeyValueColumn::ChildHashes, &hash[..])?;
            return Ok(());
        }

        let serializing = new_children
            .iter()
            .map(|x| Some(BlockHeaderHash(x.bytes()?)))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| anyhow!("invalid block header length"))?;

        let serialized = bincode::serialize(&serializing)?;

        self.inner()
            .store(KeyValueColumn::ChildHashes, &hash[..], &serialized[..])?;
        Ok(())
    }

    pub(super) fn longest_child_path(&mut self, block_hash: &Digest) -> Result<Vec<Digest>> {
        let mut round = vec![vec![block_hash.clone()]];
        let mut next_round = vec![];
        loop {
            for path in &round {
                let children = self.get_child_block_hashes(path.last().unwrap())?;
                next_round.extend(children.into_iter().map(|x| {
                    let mut path = path.clone();
                    path.push(x);
                    path
                }));
            }
            if next_round.is_empty() {
                break;
            }
            round = next_round;
            next_round = vec![];
        }

        Ok(round.into_iter().max_by_key(|x| x.len()).unwrap())
    }

    pub(super) fn insert_block(&mut self, block: SerialBlock) -> Result<()> {
        let hash = block.header.hash();
        match self.get_block_state(&hash)? {
            BlockStatus::Unknown => (),
            BlockStatus::Committed(_) | BlockStatus::Uncommitted => return Ok(()),
        }
        let mut transaction_serial_numbers = Vec::with_capacity(block.transactions.len());
        let mut transaction_commitments = Vec::with_capacity(block.transactions.len());
        let mut transaction_memos = Vec::with_capacity(block.transactions.len());

        for transaction in &block.transactions {
            transaction_serial_numbers.extend_from_slice(&transaction.old_serial_numbers[..]);
            transaction_commitments.extend_from_slice(&transaction.new_commitments[..]);
            transaction_memos.push(&transaction.memorandum);
        }

        if has_duplicates(transaction_serial_numbers) {
            return Err(StorageError::DuplicateSn.into());
        }
        if has_duplicates(transaction_commitments) {
            return Err(StorageError::DuplicateCm.into());
        }
        if has_duplicates(transaction_memos) {
            return Err(StorageError::DuplicateMemo.into());
        }

        let mut header = vec![];
        block.header.write_le(&mut header)?;
        self.inner()
            .store(KeyValueColumn::BlockHeader, &hash[..], &header[..])?;

        let mut transactions = vec![];
        block.write_transactions(&mut transactions)?;
        self.inner()
            .store(KeyValueColumn::BlockTransactions, &hash[..], &transactions[..])?;

        let mut child_hashes = self.get_child_block_hashes(&block.header.previous_block_hash)?;

        if !child_hashes.contains(&hash) {
            child_hashes.push(hash);

            self.update_child_block_hashes(&block.header.previous_block_hash, &child_hashes[..])?;
        }

        Ok(())
    }

    pub(super) fn delete_block(&mut self, hash: &Digest) -> Result<()> {
        match self.get_block_state(hash)? {
            BlockStatus::Unknown => return Ok(()),
            BlockStatus::Uncommitted => (),
            BlockStatus::Committed(_) => return Err(anyhow!("attempted to delete committed block")),
        }

        let header = self.get_block_header(hash)?;

        self.inner().delete(KeyValueColumn::BlockHeader, &hash[..])?;

        self.inner().delete(KeyValueColumn::BlockTransactions, &hash[..])?;

        let mut child_hashes = self.get_child_block_hashes(&header.previous_block_hash)?;

        if let Some(index) = child_hashes.iter().position(|x| x == hash) {
            child_hashes.remove(index);

            self.update_child_block_hashes(&header.previous_block_hash, &child_hashes[..])?;
        }

        Ok(())
    }

    pub(super) fn get_block_hash(&mut self, block_num: u32) -> Result<Option<Digest>> {
        let hash = self
            .inner()
            .get(KeyValueColumn::BlockIndex, &block_num.to_le_bytes()[..])?
            .map(|x| x[..].into());
        Ok(hash)
    }

    fn get_block_hash_guarded(&mut self, block_num: u32) -> Result<Digest> {
        let hash = self
            .inner()
            .get(KeyValueColumn::BlockIndex, &block_num.to_le_bytes()[..])?
            .map(|x| x[..].into())
            .ok_or_else(|| anyhow!("missing canon hash"))?;
        Ok(hash)
    }

    pub(super) fn get_block_header(&mut self, hash: &Digest) -> Result<SerialBlockHeader> {
        let header = self
            .inner()
            .get(KeyValueColumn::BlockHeader, &hash[..])?
            .ok_or_else(|| anyhow!("block header missing"))?;
        let header = SerialBlockHeader::read_le(&mut &header[..])?;
        Ok(header)
    }

    pub(super) fn get_block_state(&mut self, hash: &Digest) -> Result<BlockStatus> {
        let index = self.inner().get(KeyValueColumn::BlockIndex, &hash[..])?;
        match index {
            Some(index) => {
                let block_number = Self::read_u32(&index[..])?;
                Ok(BlockStatus::Committed(block_number as usize))
            }
            None => {
                if self.inner().exists(KeyValueColumn::BlockHeader, &hash[..])? {
                    Ok(BlockStatus::Uncommitted)
                } else {
                    Ok(BlockStatus::Unknown)
                }
            }
        }
    }

    pub(super) fn get_block_states(&mut self, hashes: &[Digest]) -> Result<Vec<BlockStatus>> {
        //todo: optimize this?
        hashes
            .iter()
            .map(|hash| self.get_block_state(hash))
            .collect::<Result<Vec<_>>>()
    }

    pub(super) fn get_block(&mut self, hash: &Digest) -> Result<SerialBlock> {
        let header = self.get_block_header(hash)?;
        let raw_transactions = self
            .inner()
            .get(KeyValueColumn::BlockTransactions, &hash[..])?
            .ok_or_else(|| anyhow!("missing transactions for block"))?;
        let transactions = SerialBlock::read_transactions(&mut &raw_transactions[..])?;
        Ok(SerialBlock { header, transactions })
    }

    pub(super) fn canon_height(&mut self) -> Result<u32> {
        self.read_meta_u32(KEY_BEST_BLOCK_NUMBER, Some(0))
    }

    pub(super) fn canon(&mut self) -> Result<CanonData> {
        let block_number = self.canon_height()?;
        let hash = self.get_block_hash(block_number)?;

        // handle genesis
        if hash.is_none() && block_number == 0 {
            return Ok(CanonData {
                block_height: 0,
                hash: Digest::default(), // empty
            });
        }

        Ok(CanonData {
            block_height: block_number as usize,
            hash: hash.ok_or_else(|| anyhow!("missing canon hash"))?,
        })
    }

    pub(super) fn get_block_locator_hashes(
        &mut self,
        points_of_interest: Vec<Digest>,
        oldest_fork_threshold: usize,
    ) -> Result<Vec<Digest>> {
        let canon = self.canon()?;
        let target_height = canon.block_height as u32;

        // The number of locator hashes left to obtain; accounts for the genesis block.
        let mut num_locator_hashes = std::cmp::min(crate::NUM_LOCATOR_HASHES - 1, target_height);

        // The output list of block locator hashes.
        let mut block_locator_hashes = Vec::with_capacity(num_locator_hashes as usize + points_of_interest.len());

        for hash in points_of_interest {
            trace!("block locator hash -- interesting: block# none: {}", hash);
            block_locator_hashes.push(hash);
        }

        // The index of the current block for which a locator hash is obtained.
        let mut hash_index = target_height;

        // The number of top blocks to provide locator hashes for.
        let num_top_blocks = std::cmp::min(10, num_locator_hashes);

        for _ in 0..num_top_blocks {
            let hash = self.get_block_hash_guarded(hash_index)?;
            trace!("block locator hash -- top: block# {}: {}", hash_index, hash);
            block_locator_hashes.push(hash);
            hash_index -= 1; // safe; num_top_blocks is never higher than the height
        }

        num_locator_hashes -= num_top_blocks;
        if num_locator_hashes == 0 {
            let hash = self.get_block_hash_guarded(0)?;
            trace!("block locator hash -- genesis: block# {}: {}", 0, hash);
            block_locator_hashes.push(hash);
            return Ok(block_locator_hashes);
        }

        // Calculate the average distance between block hashes based on the desired number of locator hashes.
        let mut proportional_step =
            (hash_index.min(oldest_fork_threshold as u32) / num_locator_hashes).min(crate::NUM_LOCATOR_HASHES - 1);

        // Provide hashes of blocks with indices descending quadratically while the quadratic step distance is
        // lower or close to the proportional step distance.
        let num_quadratic_steps = (proportional_step as f32).log2() as u32;

        // The remaining hashes should have a proportional index distance between them.
        let num_proportional_steps = num_locator_hashes - num_quadratic_steps;

        // Obtain a few hashes increasing the distance quadratically.
        let mut quadratic_step = 2; // the size of the first quadratic step
        for _ in 0..num_quadratic_steps {
            let hash = self.get_block_hash_guarded(hash_index)?;
            trace!("block locator hash -- quadratic: block# {}: {}", hash_index, hash);
            block_locator_hashes.push(hash);
            hash_index = hash_index.saturating_sub(quadratic_step);
            quadratic_step *= 2;
        }

        // Update the size of the proportional step so that the hashes of the remaining blocks have the same distance
        // between one another.
        proportional_step =
            (hash_index.min(oldest_fork_threshold as u32) / num_locator_hashes).min(crate::NUM_LOCATOR_HASHES - 1);

        // Tweak: in order to avoid "jumping" by too many indices with the last step,
        // increase the value of each step by 1 if the last step is too large. This
        // can result in the final number of locator hashes being a bit lower, but
        // it's preferable to having a large gap between values.
        if hash_index - proportional_step * num_proportional_steps > 2 * proportional_step {
            proportional_step += 1;
        }

        // Obtain the rest of hashes with a proportional distance between them.
        for _ in 0..num_proportional_steps {
            let hash = self.get_block_hash_guarded(hash_index)?;
            trace!("block locator hash -- proportional: block# {}: {}", hash_index, hash);
            block_locator_hashes.push(hash);
            if hash_index == 0 {
                return Ok(block_locator_hashes);
            }
            hash_index = hash_index.saturating_sub(proportional_step);
        }

        let hash = self.get_block_hash_guarded(0)?;
        trace!("block locator hash -- genesis: block# {}: {}", 0, hash);
        block_locator_hashes.push(hash);

        Ok(block_locator_hashes)
    }

    pub(super) fn find_sync_blocks(
        &mut self,
        block_locator_hashes: Vec<Digest>,
        block_count: usize,
    ) -> Result<Vec<Digest>> {
        let mut min_hash = None;
        for hash in block_locator_hashes.iter() {
            if matches!(self.get_block_state(hash)?, BlockStatus::Committed(_)) {
                min_hash = Some(hash.clone());
                break;
            }
        }
        let min_height = if let Some(min_hash) = min_hash {
            let min_height = self.get_block_state(&min_hash)?;
            match min_height {
                BlockStatus::Committed(n) => n + 1,
                _ => return Err(anyhow!("illegal block state")),
            }
        } else {
            0
        };
        let mut max_height = min_height + block_count;
        let canon = self.canon()?;
        if canon.block_height < max_height {
            max_height = canon.block_height;
        }
        let mut hashes = vec![];
        for i in min_height..=max_height {
            hashes.push(self.get_block_hash_guarded(i as u32)?);
        }
        Ok(hashes)
    }

    pub(super) fn get_transaction_location(&mut self, transaction_id: &Digest) -> Result<Option<TransactionLocation>> {
        let location = self
            .inner()
            .get(KeyValueColumn::TransactionLookup, &transaction_id[..])?;
        match location {
            Some(location) => Ok(Some(TransactionLocation::read_le(&location[..])?)),
            None => Ok(None),
        }
    }

    pub(super) fn get_canon_blocks(&mut self, limit: Option<u32>) -> Result<Vec<SerialBlock>> {
        let index = self.inner().get_column(KeyValueColumn::BlockIndex)?
            .into_iter()
            .filter(|(key, _)| key.len() == 4) // only interested in block index -> block hash maps
            .map(|(key, value)| (Self::read_u32(&key[..]).expect("invalid key"), value[..].into()))
            .collect::<Vec<(u32, Digest)>>();
        let mut blocks: HashMap<u32, SerialBlock> = HashMap::new();
        let mut max_block_number = 0u32;
        for (block_number, hash) in index {
            if let Some(limit) = limit {
                if block_number > limit {
                    continue;
                }
            }
            if block_number > max_block_number {
                max_block_number = block_number;
            }
            let block = self.get_block(&hash)?;
            blocks.insert(block_number, block);
        }
        let mut out = Vec::with_capacity(max_block_number as usize + 1);
        for i in 0..=max_block_number {
            out.push(
                blocks
                    .remove(&i)
                    .ok_or_else(|| anyhow!("missing block {} for get_canon_blocks", i))?,
            );
        }
        Ok(out)
    }

    pub(super) fn get_block_hashes(&mut self, limit: Option<u32>, filter: BlockFilter) -> Result<Vec<Digest>> {
        let mut hashes = match filter {
            BlockFilter::CanonOnly(BlockOrder::Unordered) => {
                self.inner().get_column_keys(KeyValueColumn::BlockIndex)?
                    .into_iter()
                    .filter(|key| key.len() != 4) // only interested in block hash keys
                    .map(|key| key[..].into())
                    .collect::<Vec<Digest>>()
            }
            BlockFilter::CanonOnly(order) => {
                let mut values = self.inner().get_column(KeyValueColumn::BlockIndex)?
                    .into_iter()
                    .filter(|(key, value)| key.len() != 4 && value.len() == 4) // only interested in block hash keys
                    .map(|(key, value)| (key[..].into(), u32::from_le_bytes((&value[..]).try_into().unwrap())))
                    .collect::<Vec<(Digest, u32)>>();
                values.sort_by(|a, b| a.1.cmp(&b.1));
                match order {
                    BlockOrder::Ascending => values.into_iter().map(|x| x.0).collect(),
                    BlockOrder::Descending => values.into_iter().rev().map(|x| x.0).collect(),
                    BlockOrder::Unordered => unreachable!(),
                }
            }
            BlockFilter::NonCanonOnly => {
                let all = self.get_block_hashes(None, BlockFilter::All)?;
                let canon = self
                    .get_block_hashes(None, BlockFilter::CanonOnly(BlockOrder::Unordered))?
                    .into_iter()
                    .collect::<HashSet<Digest>>();
                all.into_iter().filter(|hash| !canon.contains(hash)).collect()
            }
            BlockFilter::All => self
                .inner()
                .get_column_keys(KeyValueColumn::BlockHeader)?
                .into_iter()
                .map(|key| key[..].into())
                .collect::<Vec<Digest>>(),
        };
        // this isn't the most efficient way to do limit here, but it's good enough and clean
        if let Some(limit) = limit {
            hashes.truncate(limit as usize);
        }
        Ok(hashes)
    }
}
