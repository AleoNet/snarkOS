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
    key_value::{
        KeyValueColumn::{self, *},
        KeyValueStorage,
        KEY_BEST_BLOCK_NUMBER,
    },
    RocksDb,
    SerialBlock,
    TransactionLocation,
};

use self::LookupKind::*;

use snarkvm_dpc::{BlockHeader, BlockHeaderHash, Op};
use snarkvm_utilities::{to_bytes_le, FromBytes, ToBytes};

use tokio::{
    sync::{mpsc, oneshot},
    task,
};
use tracing::*;

use std::{
    collections::HashSet,
    convert::TryInto,
    io::Cursor,
    mem,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

macro_rules! check_for_superfluous_tx_components {
    ($fn_name:ident, $component_name:expr, $component_col:expr) => {
        async fn $fn_name(
            tx_entries: &HashSet<Vec<u8>>,
            lookup_sender: &mpsc::UnboundedSender<LookupRequest>,
            db_ops: &mut Vec<Op>,
            fix_mode: FixMode,
            is_storage_valid: &AtomicBool,
        ) {
            let storage_keys = match db_lookup($component_col, GetColumnKeys, lookup_sender).await {
                LookupResponse::Keys(col) => col.into_iter().collect::<HashSet<_>>(),
                _ => {
                    error!("Couldn't obtain the column with tx {}s", $component_name);
                    is_storage_valid.store(false, Ordering::SeqCst);

                    return;
                }
            };

            debug!("there are {} {}s stored", storage_keys.len(), $component_name);

            let superfluous_items = storage_keys.difference(tx_entries).collect::<Vec<_>>();

            if !superfluous_items.is_empty() {
                warn!(
                    "There are {} more {}s stored than there are in canon transactions",
                    superfluous_items.len(),
                    $component_name
                );

                if [FixMode::SuperfluousTestnet1TxComponents, FixMode::Everything].contains(&fix_mode) {
                    for superfluous_item in superfluous_items {
                        db_ops.push(Op::Delete {
                            col: $component_col as u32,
                            key: superfluous_item.to_vec(),
                        });
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
    Testnet1TxLocations,
    /// Store transaction serial numbers, commitments and memorandums that are missing in the storage.
    MissingTestnet1TxComponents,
    /// Remove transaction serial numbers, commitments and memorandums for missing transactions.
    SuperfluousTestnet1TxComponents,
    /// Apply all the available fixes.
    Everything,
}

#[derive(Debug)]
pub struct LookupRequest {
    col: KeyValueColumn,
    kind: LookupKind,
    tx: oneshot::Sender<LookupResponse>,
}

#[derive(Debug)]
pub enum LookupKind {
    Exists(Vec<u8>),
    Get(Vec<u8>),
    GetColumnKeys,
}

#[derive(Debug)]
pub enum LookupResponse {
    Found(bool),
    Value(Option<Vec<u8>>),
    Keys(Vec<Vec<u8>>),
}

#[derive(Debug, PartialEq)]
pub enum ValidatorAction {
    /// A transaction component from a stored transaction (as opposed to stored in its own column); the first
    /// value is the index of the component's corresponding dedicated database column, and the second one is
    /// its serialized value.
    RegisterTxComponent(KeyValueColumn, Vec<u8>),
    /// An operation that will be executed as part of a batch database transaction at the end of the validation
    /// process in case a fix mode other than `FixMode::Nothing` is picked; it will either store a missing value
    /// or delete a superfluous one.
    QueueDatabaseOp(Op),
}

check_for_superfluous_tx_components!(check_for_superfluous_tx_memos, "memorandum", Memo);

check_for_superfluous_tx_components!(check_for_superfluous_tx_digests, "digest", DigestIndex);

check_for_superfluous_tx_components!(check_for_superfluous_tx_sns, "serial number", SerialNumber);

check_for_superfluous_tx_components!(check_for_superfluous_tx_cms, "commitment", Commitment);

impl RocksDb {
    /// Validates the storage of the canon blocks, their child-parent relationships, and their transactions; starts
    /// at the current block height and goes down until the genesis block, making sure that the block-related data
    /// stored in the database is coherent. The optional limit restricts the number of blocks to check, as
    /// it is likely that any issues are applicable only to the last few blocks. The `fix` argument determines whether
    /// the validation process should also attempt to fix the issues it encounters.
    pub async fn validate(mut self, limit: Option<u32>, fix_mode: FixMode) -> Self {
        if limit.is_some() && [FixMode::SuperfluousTestnet1TxComponents, FixMode::Everything].contains(&fix_mode) {
            panic!(
                "The validator can perform the specified fixes only if there is no limit on the number of blocks to process"
            );
        }

        info!("Validating the storage...");

        if limit == Some(0) {
            info!("The limit of blocks to validate is 0; nothing to check.");
            return self;
        }

        let current_height = if let Ok(Some(height)) = self.get(Meta, KEY_BEST_BLOCK_NUMBER.as_bytes()) {
            u32::from_le_bytes(
                (&*height)
                    .try_into()
                    .expect("Invalid block height found in the storage!"),
            )
        } else {
            error!("Can't obtain block height from the storage!");
            return self;
        };

        if current_height == 0 {
            info!("Only the genesis block is currently available; nothing to check.");
            return self;
        }

        debug!("The block height is {}", current_height);

        let to_process = limit.unwrap_or(current_height);

        // Spawn a task intercepting database lookup requests and executing them sequentially.
        let (lookup_sender, mut lookup_receiver) = mpsc::unbounded_channel::<LookupRequest>();
        let lookup_task_handle = task::spawn(async move {
            while let Some(LookupRequest { col, kind, tx }) = lookup_receiver.recv().await {
                match kind {
                    Get(key) => {
                        let res = self.get(col, &key).unwrap().map(|v| v.into_owned());
                        tx.send(LookupResponse::Value(res)).unwrap()
                    }
                    Exists(key) => {
                        let res = self.exists(col, &key).unwrap();
                        tx.send(LookupResponse::Found(res)).unwrap()
                    }
                    GetColumnKeys => {
                        let res = self
                            .get_column_keys(col)
                            .unwrap()
                            .into_iter()
                            .map(|v| v.into_owned())
                            .collect();
                        tx.send(LookupResponse::Keys(res)).unwrap()
                    }
                }
            }

            self
        });

        // Spawn a task intercepting stored tx components and pending database operations from a dedicated channel.
        let (component_sender, mut component_receiver) = mpsc::unbounded_channel::<ValidatorAction>();
        let component_task_handle = task::spawn(async move {
            let mut db_ops = Vec::new();

            let mut tx_memos: HashSet<Vec<u8>> = Default::default();
            let mut tx_sns: HashSet<Vec<u8>> = Default::default();
            let mut tx_cms: HashSet<Vec<u8>> = Default::default();
            let mut tx_digests: HashSet<Vec<u8>> = Default::default();

            while let Some(action) = component_receiver.recv().await {
                match action {
                    ValidatorAction::RegisterTxComponent(col, key) => {
                        let set = match col {
                            Memo => &mut tx_memos,
                            SerialNumber => &mut tx_sns,
                            Commitment => &mut tx_cms,
                            DigestIndex => &mut tx_digests,
                            _ => unreachable!(),
                        };
                        set.insert(key);
                    }
                    ValidatorAction::QueueDatabaseOp(op) => {
                        db_ops.push(op);
                    }
                }
            }

            (db_ops, tx_memos, tx_sns, tx_cms, tx_digests)
        });

        let is_valid = Arc::new(AtomicBool::new(true));

        (0..=to_process).into_iter().for_each(|i| {
            let lookup_sender2 = lookup_sender.clone();
            let component_sender2 = component_sender.clone();
            let is_valid2 = is_valid.clone();

            task::spawn(async move {
                validate_block(
                    current_height - i,
                    lookup_sender2,
                    component_sender2,
                    fix_mode,
                    is_valid2,
                )
                .await
            });
        });

        // Close the tx component channel, breaking the loop in the task checking the receiver.
        mem::drop(component_sender);

        let (mut db_ops, tx_memos, tx_sns, tx_cms, tx_digests) = component_task_handle.await.unwrap(); // can't recover if it fails

        // Superfluous items can only be removed after a full storage pass.
        if limit.is_none() {
            check_for_superfluous_tx_memos(&tx_memos, &lookup_sender, &mut db_ops, fix_mode, &is_valid).await;
            check_for_superfluous_tx_digests(&tx_digests, &lookup_sender, &mut db_ops, fix_mode, &is_valid).await;
            check_for_superfluous_tx_sns(&tx_sns, &lookup_sender, &mut db_ops, fix_mode, &is_valid).await;
            check_for_superfluous_tx_cms(&tx_cms, &lookup_sender, &mut db_ops, fix_mode, &is_valid).await;
        }

        // Close the lookup request channel, breaking the loop in the task checking the receiver.
        mem::drop(lookup_sender);

        let mut storage = lookup_task_handle.await.unwrap();

        if fix_mode != FixMode::Nothing && !db_ops.is_empty() {
            info!("Fixing the detected storage issues");

            storage.begin().unwrap();
            for op in db_ops {
                if let Err(e) = match op {
                    Op::Insert { col, key, value } => storage.store(col.try_into().unwrap(), &key, &value),
                    Op::Delete { col, key } => storage.delete(col.try_into().unwrap(), &key),
                } {
                    error!("Couldn't fix a storage issue: {}", e);
                }
            }
            if let Err(e) = storage.commit() {
                error!("Couldn't fix the detected storage issues: {}", e);
                storage.abort().unwrap();
            }
        }

        let is_valid = is_valid.load(Ordering::SeqCst);

        if is_valid {
            info!("The storage is valid!");
        } else {
            error!("The storage is invalid!");
        }

        storage
    }
}

async fn db_lookup(
    col: KeyValueColumn,
    kind: LookupKind,
    req_sender: &mpsc::UnboundedSender<LookupRequest>,
) -> LookupResponse {
    let (resp_sender, resp_receiver) = oneshot::channel();

    let req = LookupRequest {
        col,
        kind,
        tx: resp_sender,
    };

    req_sender.send(req).unwrap();
    resp_receiver.await.unwrap()
}

/// Validates the storage of a block at the given height.
async fn validate_block(
    block_height: u32,
    lookup_sender: mpsc::UnboundedSender<LookupRequest>,
    component_sender: mpsc::UnboundedSender<ValidatorAction>,
    fix_mode: FixMode,
    is_storage_valid: Arc<AtomicBool>,
) {
    let block_hash = match db_lookup(BlockIndex, Get(block_height.to_le_bytes().to_vec()), &lookup_sender).await {
        LookupResponse::Value(Some(block_hash)) => BlockHeaderHash::new(block_hash),
        _ => {
            // Block hash not found; register the failure and attempt to carry on.
            is_storage_valid.store(false, Ordering::SeqCst);
            return;
        }
    };

    // This is extremely verbose and shouldn't be used outside of debugging.
    // trace!("Validating block at height {} ({})", block_height, block_hash);

    let block_header: BlockHeader = match db_lookup(BlockHeader, Get(block_hash.0.to_vec()), &lookup_sender).await {
        LookupResponse::Value(Some(bytes)) => FromBytes::read_le(&*bytes).unwrap(), // TODO: revise new unwraps; consider spawn_blocking()
        _ => {
            is_storage_valid.store(false, Ordering::SeqCst);
            error!("The header for block at height {} is missing!", block_height);
            return;
        }
    };

    validate_block_transactions(
        &block_hash,
        block_height,
        &lookup_sender,
        component_sender,
        fix_mode,
        &is_storage_valid,
    )
    .await;

    // The genesis block has no parent.
    if block_height == 0 {
        return;
    }

    let previous_hash = match db_lookup(
        BlockIndex,
        Get((block_height - 1).to_le_bytes().to_vec()),
        &lookup_sender,
    )
    .await
    {
        LookupResponse::Value(Some(hash)) => BlockHeaderHash::new(hash),
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

    match db_lookup(ChildHashes, Get(previous_hash.0.to_vec()), &lookup_sender).await {
        LookupResponse::Value(Some(child_hashes)) => {
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
async fn validate_block_transactions(
    block_hash: &BlockHeaderHash,
    block_height: u32,
    lookup_sender: &mpsc::UnboundedSender<LookupRequest>,
    component_sender: mpsc::UnboundedSender<ValidatorAction>,
    fix_mode: FixMode,
    is_storage_valid: &AtomicBool,
) {
    let block_stored_txs_bytes = match db_lookup(BlockTransactions, Get(block_hash.0.to_vec()), lookup_sender).await {
        LookupResponse::Value(Some(txs)) => txs,
        _ => {
            error!("Can't find the transactions stored for block {}", block_hash);
            is_storage_valid.store(false, Ordering::SeqCst);

            return;
        }
    };

    let block_stored_txs =
        task::spawn_blocking(|| SerialBlock::read_transactions(&mut Cursor::new(block_stored_txs_bytes)).unwrap())
            .await
            .unwrap();

    for (block_tx_idx, tx) in block_stored_txs.iter().enumerate() {
        for sn in &tx.old_serial_numbers {
            let sn = to_bytes_le![sn].unwrap();
            let found = db_lookup(SerialNumber, Exists(sn.clone()), lookup_sender).await;
            if !matches!(found, LookupResponse::Found(true)) {
                error!(
                    "Transaction {} doesn't have an old serial number stored",
                    hex::encode(tx.id)
                );
                is_storage_valid.store(false, Ordering::SeqCst);
            }
            component_sender
                .send(ValidatorAction::RegisterTxComponent(SerialNumber, sn))
                .unwrap();
        }

        for cm in &tx.new_commitments {
            let cm = to_bytes_le![cm].unwrap();
            let found = db_lookup(Commitment, Exists(cm.clone()), lookup_sender).await;
            if !matches!(found, LookupResponse::Found(true)) {
                error!(
                    "Transaction {} doesn't have a new commitment stored",
                    hex::encode(tx.id)
                );
                is_storage_valid.store(false, Ordering::SeqCst);
            }
            component_sender
                .send(ValidatorAction::RegisterTxComponent(Commitment, cm))
                .unwrap();
        }

        let tx_digest = to_bytes_le![tx.ledger_digest].unwrap();
        let found = db_lookup(Commitment, Exists(tx_digest.clone()), lookup_sender).await;
        if !matches!(found, LookupResponse::Found(true)) {
            warn!(
                "Transaction {} doesn't have the ledger digest stored",
                hex::encode(tx.id),
            );

            if [FixMode::MissingTestnet1TxComponents, FixMode::Everything].contains(&fix_mode) {
                let db_op = Op::Insert {
                    col: DigestIndex as u32,
                    key: tx_digest.clone(),
                    value: block_height.to_le_bytes().to_vec(),
                };
                component_sender.send(ValidatorAction::QueueDatabaseOp(db_op)).unwrap();
            } else {
                is_storage_valid.store(false, Ordering::SeqCst);
            }
        }
        component_sender
            .send(ValidatorAction::RegisterTxComponent(DigestIndex, tx_digest.to_vec()))
            .unwrap();

        let tx_memo = to_bytes_le![tx.memorandum].unwrap();
        let found = db_lookup(Memo, Exists(tx_memo.clone()), lookup_sender).await;
        if !matches!(found, LookupResponse::Found(true)) {
            error!("Transaction {} doesn't have its memo stored", hex::encode(tx.id));
            is_storage_valid.store(false, Ordering::SeqCst);
        }
        component_sender
            .send(ValidatorAction::RegisterTxComponent(Memo, tx_memo.to_vec()))
            .unwrap();

        match db_lookup(TransactionLookup, Get(tx.id.to_vec()), lookup_sender).await {
            LookupResponse::Value(Some(tx_location)) => {
                let tx_location = TransactionLocation::read_le(&*tx_location).unwrap();

                match db_lookup(BlockIndex, Get(tx_location.block_hash.to_vec()), lookup_sender).await {
                    LookupResponse::Value(Some(block_number)) => {
                        let block_number = u32::from_le_bytes((&block_number[..]).try_into().unwrap());

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

                        if [FixMode::Testnet1TxLocations, FixMode::Everything].contains(&fix_mode) {
                            let corrected_location = TransactionLocation {
                                index: block_tx_idx as u32,
                                block_hash: block_hash.0.into(),
                            };

                            let db_op = Op::Insert {
                                col: TransactionLookup as u32,
                                key: tx.id.to_vec(),
                                value: to_bytes_le!(corrected_location).unwrap(),
                            };
                            component_sender.send(ValidatorAction::QueueDatabaseOp(db_op)).unwrap();
                        } else {
                            is_storage_valid.store(false, Ordering::SeqCst);
                        }
                    }
                }
            }
            _ => {
                error!("Can't get the location of tx {}", hex::encode(tx.id));
                is_storage_valid.store(false, Ordering::SeqCst);
            }
        }
    }
}
