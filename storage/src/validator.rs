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

use crate::{
    key_value::{KeyValueColumn, KeyValueStorage, KEY_BEST_BLOCK_NUMBER},
    RocksDb,
    SerialBlock,
    TransactionLocation,
};

use snarkvm_dpc::{BlockHeader, BlockHeaderHash};
use snarkvm_utilities::{to_bytes_le, FromBytes, ToBytes};

use tokio::{sync::mpsc, task};
use tracing::*;

use std::{
    collections::HashSet,
    convert::TryInto,
    io::Cursor,
    mem,
    sync::atomic::{AtomicBool, Ordering},
};

macro_rules! check_for_superfluous_tx_components {
    ($fn_name:ident, $component_name:expr, $component_col:expr) => {
        fn $fn_name(&mut self, tx_entries: &HashSet<Vec<u8>>, fix_mode: FixMode, is_storage_valid: &AtomicBool) {
            let storage_keys = match self.get_column_keys($component_col) {
                Ok(col) => col.into_iter().map(|k| k.into_owned()).collect::<HashSet<_>>(),
                Err(e) => {
                    error!("Couldn't obtain the column with tx {}s: {}", $component_name, e);
                    is_storage_valid.store(false, Ordering::SeqCst);

                    return;
                }
            };

            let superfluous_items = storage_keys.difference(tx_entries).collect::<Vec<_>>();

            if !superfluous_items.is_empty() {
                warn!(
                    "There are {} more {}s stored than there are in canon transactions",
                    superfluous_items.len(),
                    $component_name
                );

                if [
                    FixMode::SuperfluousTestnet1TransactionComponents,
                    FixMode::Everything,
                ]
                .contains(&fix_mode)
                {
                    for superfluous_item in superfluous_items {
                        self.delete($component_col, superfluous_item).unwrap();
                    }
                } else {
                    is_storage_valid.store(false, Ordering::SeqCst);
                }
            }
        }
    };
}

#[derive(Clone, Copy, PartialEq)]
pub enum FixMode {
    /// Don't fix anything in the storage.
    Nothing,
    /// Update transaction locations if need be.
    Testnet1TransactionLocations,
    /// Store transaction serial numbers, commitments and memorandums that are missing in the storage.
    MissingTestnet1TransactionComponents,
    /// Remove transaction serial numbers, commitments and memorandums for missing transactions.
    SuperfluousTestnet1TransactionComponents,
    /// Apply all the available fixes.
    Everything,
}

#[derive(Debug, PartialEq)]
pub enum ValidatorAction {
    /// A transaction component from a stored transaction (as opposed to stored in its own column); the first
    /// value is the index of the component's corresponding dedicated database column, and the second one is
    /// its serialized value.
    RegisterTxComponent(KeyValueColumn, Vec<u8>),
}

impl RocksDb {
    check_for_superfluous_tx_components!(check_for_superfluous_tx_memos, "memorandum", KeyValueColumn::Memo);

    check_for_superfluous_tx_components!(check_for_superfluous_tx_digests, "digest", KeyValueColumn::DigestIndex);

    check_for_superfluous_tx_components!(
        check_for_superfluous_tx_sns,
        "serial number",
        KeyValueColumn::SerialNumber
    );

    check_for_superfluous_tx_components!(check_for_superfluous_tx_cms, "commitment", KeyValueColumn::Commitment);

    /// Validates the storage of the canon blocks, their child-parent relationships, and their transactions; starts
    /// at the current block height and goes down until the genesis block, making sure that the block-related data
    /// stored in the database is coherent. The optional limit restricts the number of blocks to check, as
    /// it is likely that any issues are applicable only to the last few blocks. The `fix` argument determines whether
    /// the validation process should also attempt to fix the issues it encounters.
    pub async fn validate(&mut self, limit: Option<u32>, fix_mode: FixMode) -> bool {
        if limit.is_some()
            && [FixMode::SuperfluousTestnet1TransactionComponents, FixMode::Everything].contains(&fix_mode)
        {
            panic!(
                "The validator can perform the specified fixes only if there is no limit on the number of blocks to process"
            );
        }

        info!("Validating the storage...");

        if limit == Some(0) {
            info!("The limit of blocks to validate is 0; nothing to check.");
            return true;
        }

        let current_height = if let Ok(Some(height)) = self.get(KeyValueColumn::Meta, KEY_BEST_BLOCK_NUMBER.as_bytes())
        {
            u32::from_le_bytes(
                (&*height)
                    .try_into()
                    .expect("Invalid block height found in the storage!"),
            )
        } else {
            error!("Can't obtain block height from the storage!");
            return false;
        };

        if current_height == 0 {
            info!("Only the genesis block is currently available; nothing to check.");
            return true;
        }

        debug!("The block height is {}", current_height);

        let to_process = limit.unwrap_or(current_height);

        // Spawn a task intercepting stored tx components and pending database operations from a dedicated channel.
        let (component_sender, mut component_receiver) = mpsc::unbounded_channel::<ValidatorAction>();
        let task_handle = task::spawn(async move {
            let mut tx_memos: HashSet<Vec<u8>> = Default::default();
            let mut tx_sns: HashSet<Vec<u8>> = Default::default();
            let mut tx_cms: HashSet<Vec<u8>> = Default::default();
            let mut tx_digests: HashSet<Vec<u8>> = Default::default();

            while let Some(action) = component_receiver.recv().await {
                match action {
                    ValidatorAction::RegisterTxComponent(col, key) => {
                        let set = match col {
                            KeyValueColumn::Memo => &mut tx_memos,
                            KeyValueColumn::SerialNumber => &mut tx_sns,
                            KeyValueColumn::Commitment => &mut tx_cms,
                            KeyValueColumn::DigestIndex => &mut tx_digests,
                            _ => unreachable!(),
                        };
                        set.insert(key);
                    }
                }
            }

            (tx_memos, tx_sns, tx_cms, tx_digests)
        });

        let is_valid = AtomicBool::new(true);

        (0..=to_process).into_iter().for_each(|i| {
            self.validate_block(current_height - i, component_sender.clone(), fix_mode, &is_valid);
        });

        // Close the channel, breaking the loop in the task checking the receiver.
        mem::drop(component_sender);

        let (tx_memos, tx_sns, tx_cms, tx_digests) = task_handle.await.unwrap(); // can't recover if it fails

        // Superfluous items can only be removed after a full storage pass.
        if limit.is_none() {
            self.check_for_superfluous_tx_memos(&tx_memos, fix_mode, &is_valid);
            self.check_for_superfluous_tx_digests(&tx_digests, fix_mode, &is_valid);
            self.check_for_superfluous_tx_sns(&tx_sns, fix_mode, &is_valid);
            self.check_for_superfluous_tx_cms(&tx_cms, fix_mode, &is_valid);
        }

        if fix_mode != FixMode::Nothing && self.in_transaction() {
            info!("Fixing the detected storage issues");
            if let Err(e) = self.commit() {
                error!("Couldn't fix the storage issues: {}", e);
            }
        }

        let is_valid = is_valid.load(Ordering::SeqCst);

        if is_valid {
            info!("The storage is valid!");
        } else {
            error!("The storage is invalid!");
        }

        is_valid
    }

    /// Validates the storage of a block at the given height.
    fn validate_block(
        &mut self,
        block_height: u32,
        component_sender: mpsc::UnboundedSender<ValidatorAction>,
        fix_mode: FixMode,
        is_storage_valid: &AtomicBool,
    ) {
        let block_hash = if let Ok(Some(block_hash)) = self.get(KeyValueColumn::BlockIndex, &block_height.to_le_bytes())
        {
            BlockHeaderHash::new(block_hash.into_owned())
        } else {
            // Block hash not found; register the failure and attempt to carry on.
            is_storage_valid.store(false, Ordering::SeqCst);
            return;
        };

        // This is extremely verbose and shouldn't be used outside of debugging.
        // trace!("Validating block at height {} ({})", block_height, block_hash);

        let block_header: BlockHeader = match self.get(KeyValueColumn::BlockHeader, &block_hash.0) {
            Ok(Some(bytes)) => FromBytes::read_le(&*bytes).unwrap(), // TODO: revise new unwraps
            _ => {
                is_storage_valid.store(false, Ordering::SeqCst);
                error!("The header for block at height {} is missing!", block_height);
                return;
            }
        };

        self.validate_block_transactions(&block_hash, block_height, component_sender, fix_mode, is_storage_valid);

        // The genesis block has no parent.
        if block_height == 0 {
            return;
        }

        let previous_hash = match self.get(KeyValueColumn::BlockIndex, &(block_height - 1).to_le_bytes()) {
            Ok(Some(hash)) => BlockHeaderHash::new(hash.into_owned()),
            _ => {
                error!("Couldn't find a block at height {}!", block_height - 1);
                is_storage_valid.store(false, Ordering::SeqCst);

                return;
            }
        };

        if block_header.previous_block_hash != previous_hash {
            is_storage_valid.store(false, Ordering::SeqCst);
            error!(
                "The parent hash of block at height {} doesn't match its child at {}!",
                block_height,
                block_height - 1,
            );
        }

        match self.get(KeyValueColumn::ChildHashes, &previous_hash.0) {
            Ok(Some(child_hashes)) => {
                let child_hashes: Vec<BlockHeaderHash> = bincode::deserialize(&child_hashes).unwrap();

                if !child_hashes.contains(&block_hash) {
                    is_storage_valid.store(false, Ordering::SeqCst);
                    error!(
                        "The list of children hash of block at height {} don't contain the child at {}!",
                        block_height - 1,
                        block_height,
                    );
                }
            }
            _ => {
                is_storage_valid.store(false, Ordering::SeqCst);
                error!("Can't find the children of block at height {}!", previous_hash);
            }
        }
    }

    /// Validates the storage of transactions belonging to the given block.
    fn validate_block_transactions(
        &mut self,
        block_hash: &BlockHeaderHash,
        block_height: u32,
        component_sender: mpsc::UnboundedSender<ValidatorAction>,
        fix_mode: FixMode,
        is_storage_valid: &AtomicBool,
    ) {
        let block_stored_txs_bytes = match self.get(KeyValueColumn::BlockTransactions, &block_hash.0) {
            Ok(Some(txs)) => txs,
            Ok(None) => {
                error!("Can't find the transactions stored for block {}", block_hash);
                is_storage_valid.store(false, Ordering::SeqCst);

                return;
            }
            Err(e) => {
                error!("Can't find the transactions stored for block {}: {}", block_hash, e);
                is_storage_valid.store(false, Ordering::SeqCst);

                return;
            }
        };

        let block_stored_txs = SerialBlock::read_transactions(&mut Cursor::new(&block_stored_txs_bytes[..])).unwrap();

        block_stored_txs.iter().enumerate().for_each(|(block_tx_idx, tx)| {
            for sn in &tx.old_serial_numbers {
                let sn = to_bytes_le![sn].unwrap();
                if matches!(self.exists(KeyValueColumn::SerialNumber, &sn), Ok(false) | Err(_)) {
                    error!(
                        "Transaction {} doesn't have an old serial number stored",
                        hex::encode(tx.id)
                    );
                    is_storage_valid.store(false, Ordering::SeqCst);
                }
                component_sender.send(ValidatorAction::RegisterTxComponent(KeyValueColumn::SerialNumber, sn)).unwrap();
            }

            for cm in &tx.new_commitments {
                let cm = to_bytes_le![cm].unwrap();
                if matches!(self.exists(KeyValueColumn::Commitment, &cm), Ok(false) | Err(_)) {
                    error!(
                        "Transaction {} doesn't have a new commitment stored",
                        hex::encode(tx.id)
                    );
                    is_storage_valid.store(false, Ordering::SeqCst);
                }
                component_sender.send(ValidatorAction::RegisterTxComponent(KeyValueColumn::Commitment, cm)).unwrap();
            }

            let tx_digest = to_bytes_le![tx.ledger_digest].unwrap();
            if matches!(self.exists(KeyValueColumn::DigestIndex, &tx_digest), Ok(false) | Err(_)) {
                warn!(
                    "Transaction {} doesn't have the ledger digest stored",
                    hex::encode(tx.id),
                );

                if [FixMode::MissingTestnet1TransactionComponents, FixMode::Everything].contains(&fix_mode) {
                    self.store(KeyValueColumn::DigestIndex, &tx_digest, &block_height.to_le_bytes()).unwrap();
                } else {
                    is_storage_valid.store(false, Ordering::SeqCst);
                }
            }
            component_sender.send(ValidatorAction::RegisterTxComponent(KeyValueColumn::DigestIndex, tx_digest.to_vec())).unwrap();

            let tx_memo = to_bytes_le![tx.memorandum].unwrap();
            if matches!(self.exists(KeyValueColumn::Memo, &tx_memo), Ok(false) | Err(_)) {
                error!("Transaction {} doesn't have its memo stored", hex::encode(tx.id));
                is_storage_valid.store(false, Ordering::SeqCst);
            }
            component_sender.send(ValidatorAction::RegisterTxComponent(KeyValueColumn::Memo, tx_memo.to_vec())).unwrap();

            match self.get(KeyValueColumn::TransactionLookup, &tx.id) {
                Ok(Some(tx_location)) => {
                    let tx_location = TransactionLocation::read_le(&*tx_location).unwrap();

                    match self.get(KeyValueColumn::BlockIndex, &tx_location.block_hash) {
                        Ok(Some(block_number)) => {
                            let block_number = u32::from_le_bytes((&*block_number).try_into().unwrap());

                            if block_number != block_height {
                                error!(
                                    "The block indicated by the location of tx {} doesn't match the current height ({} != {})",
                                    hex::encode(tx.id),
                                    block_number,
                                    block_height,
                                );
                                is_storage_valid.store(false, Ordering::SeqCst);
                            }
                        }
                        _ => {
                            warn!(
                                "Can't get the block number for tx {}! The block locator entry for hash {} is missing",
                                hex::encode(tx.id),
                                BlockHeaderHash(tx_location.block_hash.bytes::<32>().unwrap())
                            );

                            if [FixMode::Testnet1TransactionLocations, FixMode::Everything].contains(&fix_mode) {
                                let corrected_location = TransactionLocation {
                                    index: block_tx_idx as u32,
                                    block_hash: block_hash.0.into(),
                                };
                                self.store(KeyValueColumn::TransactionLookup, &tx.id, &to_bytes_le!(corrected_location).unwrap()).unwrap();
                            } else {
                                is_storage_valid.store(false, Ordering::SeqCst);
                            }
                        }
                    }
                },
                Err(e) => {
                    error!("Can't get the location of tx {}: {}", hex::encode(tx.id), e);
                    is_storage_valid.store(false, Ordering::SeqCst);
                }
                Ok(None) => {
                    error!("Can't get the location of tx {}", hex::encode(tx.id));
                    is_storage_valid.store(false, Ordering::SeqCst);
                }
            }
        });
    }
}
