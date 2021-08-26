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

use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
};

use anyhow::*;
use snarkvm_dpc::{
    testnet1::{instantiated::Components, Record},
    BlockHeaderHash,
    StorageError,
};
use snarkvm_utilities::{has_duplicates, FromBytes, ToBytes};
use tracing::debug;

use crate::{
    key_value::{KeyValueColumn, KEY_CURR_CM_INDEX, KEY_CURR_MEMO_INDEX, KEY_CURR_SN_INDEX},
    BlockFilter,
    BlockOrder,
    BlockStatus,
    CanonData,
    Digest,
    FixMode,
    KeyValueStorage,
    SerialBlock,
    SerialBlockHeader,
    SerialRecord,
    SerialTransaction,
    SyncStorage,
    TransactionLocation,
    VMRecord,
    Validator,
    ValidatorError,
};

use super::KEY_BEST_BLOCK_NUMBER;

pub struct KeyValueStore<S: KeyValueStorage + Validator + 'static> {
    inner: Option<S>, // the option is only for the purposes of validation, which requires ownership
}

impl<S: KeyValueStorage + Validator + 'static> KeyValueStore<S> {
    pub fn new(inner: S) -> Self {
        KeyValueStore { inner: Some(inner) }
    }

    fn inner(&mut self) -> &mut S {
        self.inner.as_mut().unwrap() // safe, as it's always available
    }

    fn read_u32(value: &[u8]) -> Result<u32> {
        if value.len() != 4 {
            return Err(anyhow!("invalid length for u32: {}"));
        }
        let mut out = [0u8; 4];
        out.copy_from_slice(value);
        Ok(u32::from_le_bytes(out))
    }

    fn read_meta_u32(&mut self, name: &str, default: Option<u32>) -> Result<u32> {
        let value = self.inner().get(KeyValueColumn::Meta, name.as_bytes())?;
        match value {
            Some(value) => Self::read_u32(&value[..]),
            None => match default {
                None => Err(anyhow!("missing meta value for {}", name)),
                Some(default) => {
                    self.inner()
                        .store(KeyValueColumn::Meta, name.as_bytes(), &default.to_le_bytes()[..])?;
                    Ok(default)
                }
            },
        }
    }

    fn write_meta_u32(&mut self, name: &str, value: u32) -> Result<()> {
        self.inner()
            .store(KeyValueColumn::Meta, name.as_bytes(), &value.to_le_bytes()[..])?;
        Ok(())
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

    fn commit_transaction(
        &mut self,
        sn_index: &mut u32,
        cm_index: &mut u32,
        memo_index: &mut u32,
        transaction: &SerialTransaction,
    ) -> Result<Vec<(Digest, u32)>> {
        let mut commitments = vec![];
        // we are leaving validation to the ledger
        for serial in transaction.old_serial_numbers.iter() {
            self.inner().store(
                KeyValueColumn::SerialNumber,
                &serial[..],
                &(*sn_index).to_le_bytes()[..],
            )?;
            *sn_index += 1;
        }

        for commitment in transaction.new_commitments.iter() {
            self.inner().store(
                KeyValueColumn::Commitment,
                &commitment[..],
                &(*cm_index).to_le_bytes()[..],
            )?;
            commitments.push((commitment.clone(), *cm_index));
            *cm_index += 1;
        }

        self.inner().store(
            KeyValueColumn::Memo,
            &transaction.memorandum[..],
            &(*memo_index).to_le_bytes()[..],
        )?;
        *memo_index += 1;

        Ok(commitments)
    }

    fn decommit_transaction(
        &mut self,
        sn_index: &mut u32,
        cm_index: &mut u32,
        memo_index: &mut u32,
        transaction: &SerialTransaction,
    ) -> Result<()> {
        for serial in transaction.old_serial_numbers.iter() {
            self.inner().delete(KeyValueColumn::SerialNumber, &serial[..])?;
            *sn_index -= 1;
        }

        for commitment in transaction.new_commitments.iter() {
            self.inner().delete(KeyValueColumn::Commitment, &commitment[..])?;
            *cm_index -= 1;
        }

        self.inner()
            .delete(KeyValueColumn::TransactionLookup, &transaction.id[..])?;

        self.inner().delete(KeyValueColumn::Memo, &transaction.memorandum[..])?;
        *memo_index -= 1;

        Ok(())
    }

    fn get_digest_keys(&mut self, column: KeyValueColumn) -> Result<Vec<Digest>> {
        let mut keys = self
            .inner()
            .get_column(column)?
            .into_iter()
            .filter(|(_, index)| index.len() == 4)
            .map(|(key, index)| (key[..].into(), Self::read_u32(&index[..]).unwrap()))
            .collect::<Vec<(Digest, u32)>>();
        keys.sort_by(|(_, a), (_, b)| a.cmp(b));
        Ok(keys.into_iter().map(|(digest, _)| digest).collect())
    }
}

impl<S: KeyValueStorage + Validator + 'static> SyncStorage for KeyValueStore<S> {
    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn get_block_children(&mut self, hash: &Digest) -> Result<Vec<Digest>> {
        Ok(self
            .inner()
            .get(KeyValueColumn::ChildHashes, &hash[..])?
            .map(|x| bincode::deserialize::<'_, Vec<BlockHeaderHash>>(&x[..]))
            .transpose()?
            .map(|x| x.into_iter().map(|x| x.0[..].into()).collect())
            .unwrap_or_else(Vec::new))
    }

    fn insert_block(&mut self, block: &SerialBlock) -> Result<()> {
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

        let mut child_hashes = self.get_block_children(&block.header.previous_block_hash)?;

        if !child_hashes.contains(&hash) {
            child_hashes.push(hash);

            self.update_child_block_hashes(&block.header.previous_block_hash, &child_hashes[..])?;
        }

        Ok(())
    }

    fn delete_block(&mut self, hash: &Digest) -> Result<()> {
        match self.get_block_state(hash)? {
            BlockStatus::Unknown => return Ok(()),
            BlockStatus::Uncommitted => (),
            BlockStatus::Committed(_) => return Err(anyhow!("attempted to delete committed block")),
        }

        let header = self.get_block_header(hash)?;

        self.inner().delete(KeyValueColumn::BlockHeader, &hash[..])?;

        self.inner().delete(KeyValueColumn::BlockTransactions, &hash[..])?;

        let mut child_hashes = self.get_block_children(&header.previous_block_hash)?;

        if let Some(index) = child_hashes.iter().position(|x| x == hash) {
            child_hashes.remove(index);

            self.update_child_block_hashes(&header.previous_block_hash, &child_hashes[..])?;
        }

        Ok(())
    }

    fn get_block_hash(&mut self, block_num: u32) -> Result<Option<Digest>> {
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

    fn get_block_header(&mut self, hash: &Digest) -> Result<SerialBlockHeader> {
        let header = self
            .inner()
            .get(KeyValueColumn::BlockHeader, &hash[..])?
            .ok_or_else(|| anyhow!("block header missing"))?;
        let header = SerialBlockHeader::read_le(&mut &header[..])?;
        Ok(header)
    }

    fn get_block_state(&mut self, hash: &Digest) -> Result<BlockStatus> {
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

    fn get_block_states(&mut self, hashes: &[Digest]) -> Result<Vec<BlockStatus>> {
        //todo: optimize this?
        hashes
            .iter()
            .map(|hash| self.get_block_state(hash))
            .collect::<Result<Vec<_>>>()
    }

    fn get_block(&mut self, hash: &Digest) -> Result<SerialBlock> {
        let header = self.get_block_header(hash)?;
        let raw_transactions = self
            .inner()
            .get(KeyValueColumn::BlockTransactions, &hash[..])?
            .ok_or_else(|| anyhow!("missing transactions for block"))?;
        let transactions = SerialBlock::read_transactions(&mut &raw_transactions[..])?;
        Ok(SerialBlock { header, transactions })
    }

    fn canon_height(&mut self) -> Result<u32> {
        self.read_meta_u32(KEY_BEST_BLOCK_NUMBER, Some(0))
    }

    fn canon(&mut self) -> Result<CanonData> {
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

    fn get_transaction_location(&mut self, transaction_id: &Digest) -> Result<Option<TransactionLocation>> {
        let location = self
            .inner()
            .get(KeyValueColumn::TransactionLookup, &transaction_id[..])?;
        match location {
            Some(location) => Ok(Some(TransactionLocation::read_le(&location[..])?)),
            None => Ok(None),
        }
    }

    fn get_canon_blocks(&mut self, limit: Option<u32>) -> Result<Vec<SerialBlock>> {
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

    fn get_block_hashes(&mut self, limit: Option<u32>, filter: BlockFilter) -> Result<Vec<Digest>> {
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

    fn commit_block(&mut self, block_hash: &Digest, ledger_digest: &Digest) -> Result<BlockStatus> {
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
            self.inner()
                .store(KeyValueColumn::TransactionLookup, &transaction.id[..], &out[..])?;
        }

        self.write_meta_u32(KEY_CURR_SN_INDEX, sn_index)?;
        self.write_meta_u32(KEY_CURR_CM_INDEX, cm_index)?;
        self.write_meta_u32(KEY_CURR_MEMO_INDEX, memo_index)?;

        let is_genesis = canon.is_empty();

        let new_best_block_number = if is_genesis { 0 } else { canon.block_height as u32 + 1 };

        self.write_meta_u32(KEY_BEST_BLOCK_NUMBER, new_best_block_number)?;

        let block_num_serialized = &new_best_block_number.to_le_bytes()[..];

        self.inner()
            .store(KeyValueColumn::BlockIndex, &block_hash[..], block_num_serialized)?;
        self.inner()
            .store(KeyValueColumn::BlockIndex, block_num_serialized, &block_hash[..])?;

        self.inner()
            .store(KeyValueColumn::DigestIndex, &ledger_digest[..], block_num_serialized)?;
        self.inner()
            .store(KeyValueColumn::DigestIndex, block_num_serialized, &ledger_digest[..])?;

        Ok(BlockStatus::Committed(new_best_block_number as usize))
    }

    fn decommit_blocks(&mut self, hash: &Digest) -> Result<Vec<SerialBlock>> {
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

            self.inner().delete(KeyValueColumn::BlockIndex, &last_hash[..])?;
            self.inner()
                .delete(KeyValueColumn::BlockIndex, block_number_serialized)?;

            let digest = self
                .inner()
                .get(KeyValueColumn::DigestIndex, block_number_serialized)?
                .ok_or_else(|| anyhow!("missing digest for block during decommiting"))?
                .into_owned();

            self.inner().delete(KeyValueColumn::DigestIndex, &digest)?;
            self.inner()
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

    fn reset_ledger(
        &mut self,
        commitments: Vec<Digest>,
        serial_numbers: Vec<Digest>,
        memos: Vec<Digest>,
        digests: Vec<Digest>,
    ) -> Result<()> {
        let mut sn_index = 0u32;
        let mut cm_index = 0u32;
        let mut memo_index = 0u32;

        self.inner().truncate(KeyValueColumn::Commitment)?;
        self.inner().truncate(KeyValueColumn::SerialNumber)?;
        self.inner().truncate(KeyValueColumn::Memo)?;
        self.inner().truncate(KeyValueColumn::DigestIndex)?;

        for commitment in commitments.into_iter() {
            self.inner()
                .store(KeyValueColumn::Commitment, &commitment[..], &cm_index.to_le_bytes()[..])?;
            cm_index += 1;
        }

        for serial_number in serial_numbers.into_iter() {
            self.inner().store(
                KeyValueColumn::SerialNumber,
                &serial_number[..],
                &sn_index.to_le_bytes()[..],
            )?;
            sn_index += 1;
        }

        for memo in memos.into_iter() {
            self.inner()
                .store(KeyValueColumn::Memo, &memo[..], &memo_index.to_le_bytes()[..])?;
            memo_index += 1;
        }

        for (i, digest) in digests.into_iter().enumerate() {
            let block_num_serialized = &(i as u32).to_le_bytes()[..];
            self.inner()
                .store(KeyValueColumn::DigestIndex, &digest[..], block_num_serialized)?;
            self.inner()
                .store(KeyValueColumn::DigestIndex, block_num_serialized, &digest[..])?;
        }

        self.write_meta_u32(KEY_CURR_SN_INDEX, sn_index)?;
        self.write_meta_u32(KEY_CURR_CM_INDEX, cm_index)?;
        self.write_meta_u32(KEY_CURR_MEMO_INDEX, memo_index)?;
        Ok(())
    }

    fn get_record_commitments(&mut self, limit: Option<usize>) -> Result<Vec<Digest>> {
        let mut records = self.inner().get_column_keys(KeyValueColumn::Records)?;
        if let Some(limit) = limit {
            records.truncate(limit);
        }
        let mut out = Vec::with_capacity(records.len());
        for record in records {
            out.push(record[..].into());
        }
        Ok(out)
    }

    fn get_record(&mut self, commitment: &Digest) -> Result<Option<SerialRecord>> {
        let raw = self.inner().get(KeyValueColumn::Records, &commitment[..])?;
        match raw {
            None => Ok(None),
            Some(record) => {
                let record = Record::<Components>::read_le(&mut &record[..])?;
                Ok(Some(<Record<Components> as VMRecord>::serialize(&record)?))
            }
        }
    }

    fn store_records(&mut self, records: &[SerialRecord]) -> Result<()> {
        for record in records {
            let mut record_data = vec![];
            record.write_le(&mut record_data)?;
            self.inner()
                .store(KeyValueColumn::Records, &record.commitment[..], &record_data[..])?;
        }
        Ok(())
    }

    fn get_ledger_digests(&mut self) -> Result<Vec<Digest>> {
        let mut keys = self
            .inner()
            .get_column(KeyValueColumn::DigestIndex)?
            .into_iter()
            .filter(|(index, _)| index.len() == 4)
            .map(|(index, key)| (key[..].into(), Self::read_u32(&index[..]).unwrap()))
            .collect::<Vec<(Digest, u32)>>();
        keys.sort_by(|(_, a), (_, b)| a.cmp(b));
        Ok(keys.into_iter().map(|(digest, _)| digest).collect())
    }

    fn validate(&mut self, limit: Option<u32>, fix_mode: FixMode) -> Vec<ValidatorError> {
        let errors = if let Some(inner) = std::mem::take(&mut self.inner) {
            let (errors, inner) = futures::executor::block_on(inner.validate(limit, fix_mode));
            self.inner = Some(inner);
            errors
        } else {
            unreachable!()
        };

        errors
    }

    #[cfg(feature = "test")]
    fn store_item(&mut self, col: KeyValueColumn, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.inner().store(col, &key, &value)
    }

    #[cfg(feature = "test")]
    fn delete_item(&mut self, col: KeyValueColumn, key: Vec<u8>) -> Result<()> {
        self.inner().delete(col, &key)
    }

    fn get_transaction(&mut self, transaction_id: &Digest) -> Result<SerialTransaction> {
        let location = self
            .get_transaction_location(transaction_id)?
            .ok_or_else(|| anyhow!("transaction not found"))?;
        let block = self.get_block(&location.block_hash)?;
        if let Some(transaction) = block.transactions.get(location.index as usize) {
            Ok(transaction.clone())
        } else {
            Err(anyhow!("missing transaction in block"))
        }
    }

    fn get_commitments(&mut self) -> Result<Vec<Digest>> {
        self.get_digest_keys(KeyValueColumn::Commitment)
    }

    fn get_serial_numbers(&mut self) -> Result<Vec<Digest>> {
        self.get_digest_keys(KeyValueColumn::SerialNumber)
    }

    fn get_memos(&mut self) -> Result<Vec<Digest>> {
        self.get_digest_keys(KeyValueColumn::Memo)
    }

    fn transact<T, F: FnOnce(&mut Self) -> Result<T>>(&mut self, func: F) -> Result<T> {
        self.inner().begin()?;
        let out = func(self);
        if out.is_err() {
            self.inner().abort()?;
        } else {
            self.inner().commit()?;
        }
        out
    }
}
