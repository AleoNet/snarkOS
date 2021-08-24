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

use std::{any::Any, fmt};

use anyhow::*;
use tokio::sync::{mpsc, oneshot};
use tracing::log::trace;
use snarkos_metrics::{self as metrics, queues};

#[cfg(feature = "test")]
use crate::key_value::KeyValueColumn;
use crate::{
    BlockFilter,
    BlockStatus,
    CanonData,
    Digest,
    FixMode,
    ForkDescription,
    SerialBlock,
    SerialBlockHeader,
    SerialRecord,
    SerialTransaction,
    Storage,
    SyncStorage,
    TransactionLocation,
    ValidatorError,
};

enum Message {
    InsertBlock(SerialBlock),
    DeleteBlock(Digest),
    GetBlockHash(u32),
    GetBlockHeader(Digest),
    GetBlockState(Digest),
    GetBlockStates(Vec<Digest>),
    GetBlock(Digest),
    GetForkPath(Digest, usize),
    CommitBlock(Digest, Digest),
    DecommitBlocks(Digest),
    Canon(),
    LongestChildPath(Digest),
    GetBlockChildren(Digest),
    GetBlockLocatorHashes(Vec<Digest>, usize), // points of interest, oldest_fork_threshold
    FindSyncBlocks(Vec<Digest>, usize),
    GetTransactionLocation(Digest),
    GetTransaction(Digest),
    GetRecordCommitments(Option<usize>),
    GetRecord(Digest),
    StoreRecords(Vec<SerialRecord>),
    GetCommitments(),
    GetSerialNumbers(),
    GetMemos(),
    GetLedgerDigests(),
    ResetLedger(Vec<Digest>, Vec<Digest>, Vec<Digest>, Vec<Digest>),
    GetCanonBlocks(Option<u32>),
    GetBlockHashes(Option<u32>, BlockFilter),
    Validate(Option<u32>, FixMode),
    StoreInitDigest(Digest),
    #[cfg(feature = "test")]
    StoreItem(KeyValueColumn, Vec<u8>, Vec<u8>),
    #[cfg(feature = "test")]
    DeleteItem(KeyValueColumn, Vec<u8>),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::InsertBlock(block) => write!(f, "InsertBlock({})", block.header.hash()),
            Message::DeleteBlock(block) => write!(f, "DeleteBlock({})", block),
            Message::GetBlockHash(block_number) => write!(f, "GetBlockHash({})", block_number),
            Message::GetBlockHeader(hash) => write!(f, "GetBlockHeader({})", hash),
            Message::GetBlockState(hash) => write!(f, "GetBlockState({})", hash),
            Message::GetBlockStates(hashes) => {
                write!(f, "GetBlockStates(")?;
                for hash in hashes {
                    write!(f, "{}, ", hash)?;
                }
                write!(f, ")")
            }
            Message::GetBlock(hash) => write!(f, "GetBlock({})", hash),
            Message::GetForkPath(hash, size) => write!(f, "GetForkPath({}, {})", hash, size),
            Message::CommitBlock(hash, ledger_digest) => write!(f, "CommitBlock({}, {})", hash, ledger_digest),
            Message::DecommitBlocks(hash) => write!(f, "DecommitBlocks({})", hash),
            Message::Canon() => write!(f, "Canon()"),
            Message::LongestChildPath(hash) => write!(f, "LongestChildPath({})", hash),
            Message::GetBlockChildren(hash) => write!(f, "GetBlockChildren({})", hash),
            Message::GetBlockLocatorHashes(canon_depth_limit, oldest_fork_threshold) => write!(
                f,
                "GetBlockLocatorHashes({:?}, {})",
                canon_depth_limit, oldest_fork_threshold
            ),
            Message::FindSyncBlocks(hashes, max_block_count) => {
                write!(f, "FindSyncBlocks(")?;
                for hash in hashes {
                    write!(f, "{}, ", hash)?;
                }
                write!(f, "{})", max_block_count)
            }
            Message::GetTransactionLocation(hash) => write!(f, "GetTransactionLocation({})", hash),
            Message::GetTransaction(hash) => write!(f, "GetTransaction({})", hash),
            Message::GetRecordCommitments(limit) => write!(f, "GetRecordCommitments({:?})", limit),
            Message::GetRecord(hash) => write!(f, "GetRecord({})", hash),
            Message::StoreRecords(records) => {
                write!(f, "StoreRecords(")?;
                for record in records {
                    write!(f, "{}, ", record.commitment)?;
                }
                write!(f, ")")
            }
            Message::GetCommitments() => write!(f, "GetCommitments()"),
            Message::GetSerialNumbers() => write!(f, "GetSerialNumbers()"),
            Message::GetMemos() => write!(f, "GetMemos()"),
            Message::GetLedgerDigests() => write!(f, "GetLedgerDigests()"),
            Message::ResetLedger(_, _, _, _) => write!(f, "ResetLedger(..)"),
            Message::GetCanonBlocks(limit) => write!(f, "GetCanonBlocks({:?})", limit),
            Message::GetBlockHashes(limit, filter) => write!(f, "GetBlockHashes({:?}, {:?})", limit, filter),
            Message::Validate(limit, fix_mode) => write!(f, "Validate({:?}, {:?})", limit, fix_mode),
            Message::StoreInitDigest(digest) => write!(f, "StoreInitDigest({})", digest),
            #[cfg(feature = "test")]
            Message::StoreItem(col, key, value) => write!(f, "StoreItem({:?}, {:?}, {:?})", col, key, value),
            #[cfg(feature = "test")]
            Message::DeleteItem(col, key) => write!(f, "DeleteItem({:?}, {:?})", col, key),
        }
    }
}

pub(super) struct Agent<S: SyncStorage + 'static> {
    inner: S, // the option is only for the purposes of validation, which requires ownership
}

impl<S: SyncStorage + 'static> Agent<S> {
    pub(super) fn new(inner: S) -> Self {
        Agent { inner }
    }

    fn wrap<T, F: FnOnce(&mut S) -> Result<T>>(&mut self, func: F) -> Result<T> {
        self.inner.transact(func)
    }

    fn handle_message(&mut self, message: Message) -> Box<dyn Any + Send + Sync> {
        trace!("received message: {}", message);
        match message {
            Message::InsertBlock(block) => Box::new(self.wrap(move |f| f.insert_block(&block))),
            Message::DeleteBlock(hash) => Box::new(self.wrap(move |f| f.delete_block(&hash))),
            Message::GetBlockHash(block_num) => Box::new(self.inner.get_block_hash(block_num)),
            Message::GetBlockHeader(hash) => Box::new(self.inner.get_block_header(&hash)),
            Message::GetBlockState(hash) => Box::new(self.inner.get_block_state(&hash)),
            Message::GetBlockStates(hashes) => Box::new(self.inner.get_block_states(&hashes[..])),
            Message::GetBlock(hash) => Box::new(self.inner.get_block(&hash)),
            Message::GetForkPath(hash, oldest_fork_threshold) => {
                Box::new(self.inner.get_fork_path(&hash, oldest_fork_threshold))
            }
            Message::CommitBlock(block_hash, ledger_digest) => {
                Box::new(self.wrap(move |f| f.commit_block(&block_hash, &ledger_digest)))
            }
            Message::DecommitBlocks(hash) => Box::new(self.wrap(move |f| f.decommit_blocks(&hash))),
            Message::Canon() => Box::new(self.inner.canon()),
            Message::GetBlockChildren(hash) => Box::new(self.inner.get_block_children(&hash)),
            Message::LongestChildPath(hash) => Box::new(self.inner.longest_child_path(&hash)),
            Message::GetBlockLocatorHashes(points_of_interest, oldest_fork_threshold) => Box::new(
                self.inner
                    .get_block_locator_hashes(points_of_interest, oldest_fork_threshold),
            ),
            Message::FindSyncBlocks(hashes, block_count) => Box::new(self.inner.find_sync_blocks(hashes, block_count)),
            Message::GetTransactionLocation(transaction_id) => {
                Box::new(self.inner.get_transaction_location(&transaction_id))
            }
            Message::GetTransaction(transaction_id) => Box::new(self.inner.get_transaction(&transaction_id)),
            Message::GetRecordCommitments(limit) => Box::new(self.inner.get_record_commitments(limit)),
            Message::GetRecord(commitment) => Box::new(self.inner.get_record(&commitment)),
            Message::StoreRecords(records) => Box::new(self.wrap(move |f| f.store_records(&records[..]))),
            Message::GetCommitments() => Box::new(self.inner.get_commitments()),
            Message::GetSerialNumbers() => Box::new(self.inner.get_serial_numbers()),
            Message::GetMemos() => Box::new(self.inner.get_memos()),
            Message::GetLedgerDigests() => Box::new(self.inner.get_ledger_digests()),
            Message::ResetLedger(commitments, serial_numbers, memos, digests) => {
                Box::new(self.wrap(move |f| f.reset_ledger(commitments, serial_numbers, memos, digests)))
            }
            Message::GetCanonBlocks(limit) => Box::new(self.inner.get_canon_blocks(limit)),
            Message::GetBlockHashes(limit, filter) => Box::new(self.inner.get_block_hashes(limit, filter)),
            Message::Validate(limit, fix_mode) => Box::new(self.inner.validate(limit, fix_mode)),
            Message::StoreInitDigest(digest) => Box::new(self.wrap(move |f| f.store_init_digest(&digest))),
            #[cfg(feature = "test")]
            Message::StoreItem(col, key, value) => Box::new(self.wrap(move |f| f.store_item(col, key, value))),
            #[cfg(feature = "test")]
            Message::DeleteItem(col, key) => Box::new(self.wrap(move |f| f.delete_item(col, key))),
        }
    }

    fn agent(mut self, mut receiver: mpsc::Receiver<MessageWrapper>) {
        self.inner.init().expect("failed to initialize sync storage");
        while let Some((message, response)) = receiver.blocking_recv() {
            metrics::decrement_gauge!(queues::STORAGE, 1.0);
            let out = self.handle_message(message);
            response.send(out).ok();
        }
    }
}

type MessageWrapper = (Message, oneshot::Sender<Box<dyn Any + Send + Sync>>);

pub struct AsyncStorage {
    sender: mpsc::Sender<MessageWrapper>,
}

impl AsyncStorage {
    pub fn new<S: SyncStorage + Send + 'static>(inner: S) -> AsyncStorage {
        let (sender, receiver) = mpsc::channel(256);
        tokio::task::spawn_blocking(move || Agent::new(inner).agent(receiver));
        Self { sender }
    }

    #[allow(clippy::ok_expect)]
    async fn send<T: Send + Sync + 'static>(&self, message: Message) -> T {
        let (sender, receiver) = oneshot::channel();
        metrics::increment_gauge!(queues::STORAGE, 1.0);
        self.sender.send((message, sender)).await.ok();
        *receiver
            .await
            .ok()
            .expect("storage handler missing")
            .downcast()
            .expect("type mismatch for async adapter store handle")
    }
}

#[async_trait::async_trait]
impl Storage for AsyncStorage {
    async fn insert_block(&self, block: &SerialBlock) -> Result<()> {
        self.send(Message::InsertBlock(block.clone())).await
    }

    async fn delete_block(&self, hash: &Digest) -> Result<()> {
        self.send(Message::DeleteBlock(hash.clone())).await
    }

    async fn get_block_hash(&self, block_num: u32) -> Result<Option<Digest>> {
        self.send(Message::GetBlockHash(block_num)).await
    }

    async fn get_block_header(&self, hash: &Digest) -> Result<SerialBlockHeader> {
        self.send(Message::GetBlockHeader(hash.clone())).await
    }

    async fn get_block_state(&self, hash: &Digest) -> Result<BlockStatus> {
        self.send(Message::GetBlockState(hash.clone())).await
    }

    async fn get_block_states(&self, hashes: &[Digest]) -> Result<Vec<BlockStatus>> {
        self.send(Message::GetBlockStates(hashes.to_vec())).await
    }

    async fn get_block(&self, hash: &Digest) -> Result<SerialBlock> {
        self.send(Message::GetBlock(hash.clone())).await
    }

    async fn get_fork_path(&self, hash: &Digest, oldest_fork_threshold: usize) -> Result<ForkDescription> {
        self.send(Message::GetForkPath(hash.clone(), oldest_fork_threshold))
            .await
    }

    async fn commit_block(&self, hash: &Digest, digest: Digest) -> Result<BlockStatus> {
        self.send(Message::CommitBlock(hash.clone(), digest)).await
    }

    async fn decommit_blocks(&self, hash: &Digest) -> Result<Vec<SerialBlock>> {
        self.send(Message::DecommitBlocks(hash.clone())).await
    }

    async fn canon(&self) -> Result<CanonData> {
        self.send(Message::Canon()).await
    }

    async fn longest_child_path(&self, block_hash: &Digest) -> Result<Vec<Digest>> {
        self.send(Message::LongestChildPath(block_hash.clone())).await
    }

    async fn get_block_children(&self, block_hash: &Digest) -> Result<Vec<Digest>> {
        self.send(Message::GetBlockChildren(block_hash.clone())).await
    }

    async fn get_block_locator_hashes(
        &self,
        points_of_interest: Vec<Digest>,
        oldest_fork_threshold: usize,
    ) -> Result<Vec<Digest>> {
        self.send(Message::GetBlockLocatorHashes(
            points_of_interest,
            oldest_fork_threshold,
        ))
        .await
    }

    async fn find_sync_blocks(&self, block_locator_hashes: &[Digest], block_count: usize) -> Result<Vec<Digest>> {
        self.send(Message::FindSyncBlocks(block_locator_hashes.to_vec(), block_count))
            .await
    }

    async fn get_transaction_location(&self, transaction_id: Digest) -> Result<Option<TransactionLocation>> {
        self.send(Message::GetTransactionLocation(transaction_id)).await
    }

    async fn get_transaction(&self, transaction_id: Digest) -> Result<SerialTransaction> {
        self.send(Message::GetTransaction(transaction_id)).await
    }

    async fn get_record_commitments(&self, limit: Option<usize>) -> Result<Vec<Digest>> {
        self.send(Message::GetRecordCommitments(limit)).await
    }

    async fn get_record(&self, commitment: Digest) -> Result<Option<SerialRecord>> {
        self.send(Message::GetRecord(commitment)).await
    }

    async fn store_records(&self, records: &[SerialRecord]) -> Result<()> {
        self.send(Message::StoreRecords(records.to_vec())).await
    }

    async fn get_commitments(&self) -> Result<Vec<Digest>> {
        self.send(Message::GetCommitments()).await
    }

    async fn get_serial_numbers(&self) -> Result<Vec<Digest>> {
        self.send(Message::GetSerialNumbers()).await
    }

    async fn get_memos(&self) -> Result<Vec<Digest>> {
        self.send(Message::GetMemos()).await
    }

    async fn get_ledger_digests(&self) -> Result<Vec<Digest>> {
        self.send(Message::GetLedgerDigests()).await
    }

    async fn reset_ledger(
        &self,
        commitments: Vec<Digest>,
        serial_numbers: Vec<Digest>,
        memos: Vec<Digest>,
        digests: Vec<Digest>,
    ) -> Result<()> {
        self.send(Message::ResetLedger(commitments, serial_numbers, memos, digests))
            .await
    }

    async fn get_canon_blocks(&self, limit: Option<u32>) -> Result<Vec<SerialBlock>> {
        self.send(Message::GetCanonBlocks(limit)).await
    }

    async fn get_block_hashes(&self, limit: Option<u32>, filter: BlockFilter) -> Result<Vec<Digest>> {
        self.send(Message::GetBlockHashes(limit, filter)).await
    }

    async fn validate(&self, limit: Option<u32>, fix_mode: FixMode) -> Vec<ValidatorError> {
        self.send(Message::Validate(limit, fix_mode)).await
    }

    async fn store_init_digest(&self, digest: Digest) -> Result<()> {
        self.send(Message::StoreInitDigest(digest)).await
    }

    #[cfg(feature = "test")]
    async fn store_item(&self, col: KeyValueColumn, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.send(Message::StoreItem(col, key, value)).await
    }

    #[cfg(feature = "test")]
    async fn delete_item(&self, col: KeyValueColumn, key: Vec<u8>) -> Result<()> {
        self.send(Message::DeleteItem(col, key)).await
    }
}
