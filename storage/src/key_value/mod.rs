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

use std::{any::Any, borrow::Cow, fmt};

use tokio::sync::{mpsc, oneshot};

use crate::{BlockFilter, Digest, SerialBlock, SerialRecord};
use anyhow::*;

mod storage;
pub use storage::*;

mod column;
pub use column::*;

mod agent;
use agent::Agent;

pub type Value<'a> = Cow<'a, [u8]>;

pub trait KeyValueStorage {
    fn get<'a>(&'a mut self, column: KeyValueColumn, key: &[u8]) -> Result<Option<Value<'a>>>;

    fn exists(&mut self, column: KeyValueColumn, key: &[u8]) -> Result<bool>;

    fn get_column_keys<'a>(&'a mut self, column: KeyValueColumn) -> Result<Vec<Value<'a>>>;

    fn get_column<'a>(&'a mut self, column: KeyValueColumn) -> Result<Vec<(Value<'a>, Value<'a>)>>;

    fn store(&mut self, column: KeyValueColumn, key: &[u8], value: &[u8]) -> Result<()>;

    fn delete(&mut self, column: KeyValueColumn, key: &[u8]) -> Result<()>;

    fn begin(&mut self) -> Result<()>;

    fn abort(&mut self) -> Result<()>;

    fn commit(&mut self) -> Result<()>;

    fn truncate(&mut self, column: KeyValueColumn) -> Result<()> {
        let keys = self
            .get_column_keys(column)?
            .into_iter()
            .map(|x| x.into_owned())
            .collect::<Vec<_>>();
        for key in keys {
            self.delete(column, &key[..])?;
        }
        Ok(())
    }
}

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
    GetBlockLocatorHashes(),
    FindSyncBlocks(Vec<Digest>, usize),
    GetTransactionLocation(Digest),
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
            Message::GetBlockLocatorHashes() => write!(f, "GetBlockLocatorHashes()"),
            Message::FindSyncBlocks(hashes, max_block_count) => {
                write!(f, "FindSyncBlocks(")?;
                for hash in hashes {
                    write!(f, "{}, ", hash)?;
                }
                write!(f, "{})", max_block_count)
            }
            Message::GetTransactionLocation(hash) => write!(f, "GetTransactionLocation({})", hash),
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
        }
    }
}

type MessageWrapper = (Message, oneshot::Sender<Box<dyn Any + Send + Sync>>);

pub struct KeyValueStore {
    sender: mpsc::Sender<MessageWrapper>,
}

impl KeyValueStore {
    pub fn new<S: KeyValueStorage + Send + 'static>(inner: S) -> KeyValueStore {
        let (sender, receiver) = mpsc::channel(256);
        tokio::task::spawn_blocking(move || Agent::new(inner).agent(receiver));
        Self { sender }
    }

    #[allow(clippy::ok_expect)]
    async fn send<T: Send + Sync + 'static>(&self, message: Message) -> T {
        let (sender, receiver) = oneshot::channel();
        self.sender.send((message, sender)).await.ok();
        *receiver
            .await
            .ok()
            .expect("storage handler missing")
            .downcast()
            .expect("type mismatch for key value store handle")
    }
}
