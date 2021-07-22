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
    any::Any,
    collections::{HashMap, VecDeque},
};

use snarkvm_dpc::{
    testnet1::{instantiated::Components, Record},
    BlockHeaderHash,
    StorageError,
};
use snarkvm_utilities::{has_duplicates, FromBytes, ToBytes};
use tokio::sync::mpsc;
use tracing::log::trace;

use crate::{
    key_value::*,
    BlockStatus,
    CanonData,
    Digest,
    ForkDescription,
    ForkPath,
    KeyValueStorage,
    SerialBlock,
    SerialBlockHeader,
    SerialTransaction,
    TransactionLocation,
    VMRecord,
};

mod block;
mod block_commit;

pub(super) struct Agent<S: KeyValueStorage + 'static> {
    inner: S,
}

impl<S: KeyValueStorage + 'static> Agent<S> {
    pub(super) fn new(inner: S) -> Self {
        Agent { inner }
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
        let value = self.inner.get(KeyValueColumn::Meta, name.as_bytes())?;
        match value {
            Some(value) => Self::read_u32(&value[..]),
            None => match default {
                None => Err(anyhow!("missing meta value for {}", name)),
                Some(default) => {
                    self.inner
                        .store(KeyValueColumn::Meta, name.as_bytes(), &default.to_le_bytes()[..])?;
                    Ok(default)
                }
            },
        }
    }

    fn write_meta_u32(&mut self, name: &str, value: u32) -> Result<()> {
        self.inner
            .store(KeyValueColumn::Meta, name.as_bytes(), &value.to_le_bytes()[..])?;
        Ok(())
    }

    fn get_record_commitments(&mut self, limit: Option<usize>) -> Result<Vec<Digest>> {
        let mut records = self.inner.get_column_keys(KeyValueColumn::Records)?;
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
        let raw = self.inner.get(KeyValueColumn::Records, &commitment[..])?;
        match raw {
            None => Ok(None),
            Some(record) => {
                let record = Record::<Components>::read_le(&mut &record[..])?;
                Ok(Some(<Record<Components> as VMRecord>::serialize(&record)?))
            }
        }
    }

    fn store_records(&mut self, records: Vec<SerialRecord>) -> Result<()> {
        for record in records {
            let mut record_data = vec![];
            record.write_le(&mut record_data)?;
            self.inner
                .store(KeyValueColumn::Records, &record.commitment[..], &record_data[..])?;
        }
        Ok(())
    }

    fn get_digest_keys(&mut self, column: KeyValueColumn) -> Result<Vec<Digest>> {
        let mut keys = self
            .inner
            .get_column(column)?
            .into_iter()
            .filter(|(_, index)| index.len() == 4)
            .map(|(key, index)| (key[..].into(), Self::read_u32(&index[..]).unwrap()))
            .collect::<Vec<(Digest, u32)>>();
        keys.sort_by(|(_, a), (_, b)| a.cmp(b));
        Ok(keys.into_iter().map(|(digest, _)| digest).collect())
    }

    fn get_ledger_digests(&mut self) -> Result<Vec<Digest>> {
        let mut keys = self
            .inner
            .get_column(KeyValueColumn::DigestIndex)?
            .into_iter()
            .filter(|(index, _)| index.len() == 4)
            .map(|(index, key)| (key[..].into(), Self::read_u32(&index[..]).unwrap()))
            .collect::<Vec<(Digest, u32)>>();
        keys.sort_by(|(_, a), (_, b)| a.cmp(b));
        Ok(keys.into_iter().map(|(digest, _)| digest).collect())
    }

    fn wrap<T, F: FnOnce(&mut Self) -> Result<T>>(&mut self, func: F) -> Result<T> {
        self.inner.begin()?;
        let out = func(self);
        if out.is_err() {
            self.inner.abort()?;
        } else {
            self.inner.commit()?;
        }
        out
    }

    fn handle_message(&mut self, message: Message) -> Box<dyn Any + Send + Sync> {
        trace!("received message: {}", message);
        match message {
            Message::InsertBlock(block) => Box::new(self.wrap(move |f| f.insert_block(block))),
            Message::DeleteBlock(hash) => Box::new(self.wrap(move |f| f.delete_block(&hash))),
            Message::GetBlockHash(block_num) => Box::new(self.get_block_hash(block_num)),
            Message::GetBlockHeader(hash) => Box::new(self.get_block_header(&hash)),
            Message::GetBlockState(hash) => Box::new(self.get_block_state(&hash)),
            Message::GetBlockStates(hashes) => Box::new(self.get_block_states(&hashes[..])),
            Message::GetBlock(hash) => Box::new(self.get_block(&hash)),
            Message::GetForkPath(hash, oldest_fork_threshold) => {
                Box::new(self.get_fork_path(&hash, oldest_fork_threshold))
            }
            Message::CommitBlock(block_hash, ledger_digest) => {
                Box::new(self.wrap(move |f| f.commit_block(&block_hash, &ledger_digest)))
            }
            Message::DecommitBlocks(hash) => Box::new(self.wrap(move |f| f.decommit_blocks(&hash))),
            Message::Canon() => Box::new(self.canon()),
            Message::LongestChildPath(hash) => Box::new(self.longest_child_path(&hash)),
            Message::GetBlockLocatorHashes() => Box::new(self.get_block_locator_hashes()),
            Message::FindSyncBlocks(hashes, block_count) => Box::new(self.find_sync_blocks(hashes, block_count)),
            Message::GetTransactionLocation(transaction_id) => Box::new(self.get_transaction_location(&transaction_id)),
            Message::GetRecordCommitments(limit) => Box::new(self.get_record_commitments(limit)),
            Message::GetRecord(commitment) => Box::new(self.get_record(&commitment)),
            Message::StoreRecords(records) => Box::new(self.wrap(move |f| f.store_records(records))),
            Message::GetCommitments() => Box::new(self.get_digest_keys(KeyValueColumn::Commitment)),
            Message::GetSerialNumbers() => Box::new(self.get_digest_keys(KeyValueColumn::SerialNumber)),
            Message::GetMemos() => Box::new(self.get_digest_keys(KeyValueColumn::Memo)),
            Message::GetLedgerDigests() => Box::new(self.get_ledger_digests()),
            Message::ResetLedger(commitments, serial_numbers, memos, digests) => {
                Box::new(self.wrap(move |f| f.reset_ledger(commitments, serial_numbers, memos, digests)))
            }
            Message::GetCanonBlocks(limit) => Box::new(self.get_canon_blocks(limit)),
            Message::GetBlockHashes(limit, filter) => Box::new(self.get_block_hashes(limit, filter)),
        }
    }

    pub(super) fn agent(mut self, mut receiver: mpsc::Receiver<MessageWrapper>) {
        while let Some((message, response)) = receiver.blocking_recv() {
            let out = self.handle_message(message);
            response.send(out).ok();
        }
    }
}
