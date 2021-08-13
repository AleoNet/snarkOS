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
    Storage,
    TransactionLocation,
};
use anyhow::*;

use super::{KeyValueStore, Message};

#[async_trait::async_trait]
impl Storage for KeyValueStore {
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

    async fn get_block_locator_hashes(&self) -> Result<Vec<Digest>> {
        self.send(Message::GetBlockLocatorHashes()).await
    }

    async fn find_sync_blocks(&self, block_locator_hashes: &[Digest], block_count: usize) -> Result<Vec<Digest>> {
        self.send(Message::FindSyncBlocks(block_locator_hashes.to_vec(), block_count))
            .await
    }

    async fn get_transaction_location(&self, transaction_id: Digest) -> Result<Option<TransactionLocation>> {
        self.send(Message::GetTransactionLocation(transaction_id)).await
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

    async fn validate(&self, limit: Option<u32>, fix_mode: FixMode) -> bool {
        self.send(Message::Validate(limit, fix_mode)).await
    }

    async fn store_init_digest(&self, digest: Digest) -> Result<()> {
        self.send(Message::StoreInitDigest(digest)).await
    }

    #[cfg(feature = "test")]
    async fn delete_item(&self, col: KeyValueColumn, key: Vec<u8>) -> Result<()> {
        self.send(Message::DeleteItem(col, key)).await
    }
}
