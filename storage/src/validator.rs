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

use crate::{Ledger, TransactionLocation, COL_COMMITMENT, COL_MEMO, COL_SERIAL_NUMBER, COL_TRANSACTION_LOCATION};
use snarkvm_algorithms::traits::LoadableMerkleParameters;
use snarkvm_dpc::{Block, BlockHeaderHash, DatabaseTransaction, LedgerScheme, Op, Storage, TransactionScheme};
use snarkvm_utilities::{to_bytes, ToBytes};

use tracing::*;

use std::collections::HashSet;

macro_rules! validate_tx_components {
    ($fn_name:ident, $component_name:expr, $component_col:expr) => {
        fn $fn_name(
            &self,
            tx_entries: &HashSet<Vec<u8>>,
            database_fix: &mut Option<DatabaseTransaction>,
            is_storage_valid: &mut bool,
        ) {
            let storage_entries_and_indices = match self.storage.get_col($component_col) {
                Ok(col) => col,
                Err(e) => {
                    error!("Couldn't obtain the column with tx {}s: {}", $component_name, e);
                    *is_storage_valid = false;

                    return;
                }
            };

            let storage_entries = storage_entries_and_indices
                .into_iter()
                .map(|(entry, _)| entry.into_vec())
                .collect::<HashSet<_>>();

            let superfluous_items = storage_entries.difference(&tx_entries).collect::<Vec<_>>();

            if !superfluous_items.is_empty() {
                warn!(
                    "There are {} more {}s stored than there are in canon transactions",
                    superfluous_items.len(),
                    $component_name
                );

                if let Some(ref mut fix) = database_fix {
                    for superfluous_item in superfluous_items {
                        trace!("Staging a {} for deletion", $component_name);
                        fix.push(Op::Delete {
                            col: $component_col,
                            key: superfluous_item.to_vec(),
                        });
                    }
                } else {
                    *is_storage_valid = false;
                }
            }
        }
    };
}

impl<T: TransactionScheme, P: LoadableMerkleParameters, S: Storage> Ledger<T, P, S> {
    validate_tx_components!(validate_transaction_memos, "memorandums", COL_MEMO);

    validate_tx_components!(validate_transaction_sns, "serial numbers", COL_SERIAL_NUMBER);

    validate_tx_components!(validate_transaction_cms, "commitments", COL_COMMITMENT);

    /// Validates the storage of the canon blocks, their child-parent relationships, and their transactions; starts
    /// at the current block height and goes down until the genesis block, making sure that the block-related data
    /// stored in the database is coherent. The optional limit restricts the number of blocks to check, as
    /// it is likely that any issues are applicable only to the last few blocks. The `fix` argument determines whether
    /// the validation process should also attempt to fix the issues it encounters.
    pub fn validate(&self, mut limit: Option<usize>, fix: bool) -> bool {
        if limit.is_some() && fix {
            panic!("The validator can perform fixes only if there is no limit on the number of blocks to process");
        }

        info!("Validating the storage...");

        let mut is_valid = true;

        if limit == Some(0) {
            info!("The limit of blocks to validate is 0; nothing to check.");

            return is_valid;
        }

        let mut database_fix = if fix { Some(DatabaseTransaction::new()) } else { None };

        let mut current_height = self.get_current_block_height();

        if current_height == 0 {
            info!("Only the genesis block is currently available; nothing to check.");

            return is_valid;
        }

        debug!("The block height is {}", current_height);

        match self.get_best_block_number() {
            Err(_) => {
                is_valid = false;
                error!("Can't obtain the best block number from storage!");
            }
            Ok(number) => {
                // Initial block height comes from KEY_BEST_BLOCK_NUMBER.
                if number != current_height {
                    is_valid = false;
                    error!("Current best block number doesn't match the block height!");
                }
            }
        }

        let mut true_height_mismatch = false;

        // Current block is found by COL_BLOCK_LOCATOR, as it should have been committed (i.e. be canon).
        let mut current_block = self.get_block_from_block_number(current_height);
        while let Err(e) = current_block {
            is_valid = false;
            error!(
                "Couldn't find the latest block (height {}): {}! Trying a lower height next.",
                current_height, e
            );

            true_height_mismatch = true;

            current_height -= 1;

            if let Some(ref mut limit) = limit {
                *limit -= 1;
                if *limit == 0 {
                    info!("Specified block limit reached; the check is complete.");

                    return is_valid;
                }
            }

            current_block = self.get_block_from_block_number(current_height);
        }

        let mut current_block = match current_block {
            Ok(block) => block,
            Err(e) => {
                error!("Couldn't even obtain the genesis block by height 0: {}!", e);
                error!("The storage is invalid!");

                return false;
            }
        };

        if true_height_mismatch {
            debug!("The true block height is {}", current_height);
        }

        let mut tx_memos = HashSet::new();
        let mut tx_sns = HashSet::new();
        let mut tx_cms = HashSet::new();

        let mut current_hash = current_block.header.get_hash();

        loop {
            trace!("Validating block at height {} ({})", current_height, current_hash);

            if current_height % 100 == 0 {
                debug!("Still validating; current height: {}", current_height);
            }

            if !self.block_hash_exists(&current_hash) {
                is_valid = false;
                error!("The header for block at height {} is missing!", current_height);
            }

            self.validate_block_transactions(
                &current_block,
                current_height,
                &mut tx_memos,
                &mut tx_sns,
                &mut tx_cms,
                &mut database_fix,
                &mut is_valid,
            );

            if current_height == 0 {
                break;
            }

            current_height -= 1;

            let previous_block = match self.get_block_from_block_number(current_height) {
                Ok(block) => block,
                Err(e) => {
                    error!("Couldn't find a block at height {}: {}!", current_height, e);
                    error!("The storage is invalid!");

                    return false;
                }
            };

            let previous_hash = previous_block.header.get_hash();

            if current_block.header.previous_block_hash != previous_hash {
                is_valid = false;
                error!(
                    "The parent hash of block at height {} doesn't match its child at {}!",
                    current_height + 1,
                    current_height,
                );
            }

            match self.get_child_block_hashes(&previous_hash) {
                Err(e) => {
                    is_valid = false;
                    error!("Can't find the children of block at height {}: {}!", previous_hash, e);
                }
                Ok(child_hashes) => {
                    if !child_hashes.contains(&current_hash) {
                        is_valid = false;
                        error!(
                            "The list of children hash of block at height {} don't contain the child at {}!",
                            current_height,
                            current_height + 1,
                        );
                    }
                }
            }

            if let Some(ref mut limit) = limit {
                *limit -= 1;
                if *limit == 0 {
                    info!("Specified block limit reached; the check is complete.");

                    break;
                }
            }

            current_block = previous_block;
            current_hash = previous_hash;
        }

        self.validate_transaction_memos(&tx_memos, &mut database_fix, &mut is_valid);
        self.validate_transaction_sns(&tx_sns, &mut database_fix, &mut is_valid);
        self.validate_transaction_cms(&tx_cms, &mut database_fix, &mut is_valid);

        if let Some(fix) = database_fix {
            if !fix.0.is_empty() {
                info!("Fixing the storage issues");
                if let Err(e) = self.storage.batch(fix) {
                    error!("Couldn't fix the storage issues: {}", e);
                }
            }
        }

        if is_valid {
            info!("The storage is valid!");
        } else {
            error!("The storage is invalid!");
        }

        is_valid
    }

    /// Validates the storage of transactions belonging to the given block.
    fn validate_block_transactions(
        &self,
        block: &Block<T>,
        block_height: u32,
        tx_memos: &mut HashSet<Vec<u8>>,
        tx_sns: &mut HashSet<Vec<u8>>,
        tx_cms: &mut HashSet<Vec<u8>>,
        database_fix: &mut Option<DatabaseTransaction>,
        is_storage_valid: &mut bool,
    ) {
        for (block_tx_idx, tx) in block.transactions.iter().enumerate() {
            let tx_id = match tx.transaction_id() {
                Ok(hash) => hash,
                Err(e) => {
                    error!(
                        "The id of a transaction from block {} can't be parsed: {}",
                        block.header.get_hash(),
                        e
                    );
                    *is_storage_valid = false;

                    continue;
                }
            };

            let tx = match self.get_transaction_bytes(&tx_id) {
                Ok(tx) => match T::read(&tx[..]) {
                    Ok(tx) => tx,
                    Err(e) => {
                        error!("Transaction {} can't be parsed: {}", hex::encode(tx_id), e);
                        *is_storage_valid = false;

                        continue;
                    }
                },
                Err(e) => {
                    error!(
                        "Transaction {} can't be found in the storage: {}",
                        hex::encode(tx_id),
                        e
                    );
                    *is_storage_valid = false;

                    continue;
                }
            };

            for sn in tx.old_serial_numbers() {
                if !self.contains_sn(&sn) {
                    error!(
                        "Transaction {} doesn't have an old serial number stored",
                        hex::encode(tx_id)
                    );
                    *is_storage_valid = false;
                }
                tx_sns.insert(to_bytes!(sn).unwrap()); // to_bytes can't fail
            }

            for cm in tx.new_commitments() {
                if !self.contains_cm(&cm) {
                    error!(
                        "Transaction {} doesn't have a new commitment stored",
                        hex::encode(tx_id)
                    );
                    *is_storage_valid = false;
                }
                tx_cms.insert(to_bytes!(cm).unwrap()); // to_bytes can't fail
            }

            let tx_memo = tx.memorandum();
            if !self.contains_memo(&tx_memo) {
                error!("Transaction {} doesn't have its memo stored", hex::encode(tx_id));
                *is_storage_valid = false;
            }
            tx_memos.insert(to_bytes!(tx_memo).unwrap()); // to_bytes can't fail

            match self.get_transaction_location(&tx_id) {
                Ok(Some(tx_location)) => match self.get_block_number(&BlockHeaderHash(tx_location.block_hash)) {
                    Ok(block_number) => {
                        if block_number != block_height {
                            error!(
                                "The block indicated by the location of tx {} doesn't match the current height ({} != {})",
                                hex::encode(tx_id),
                                block_number,
                                block_height,
                            );
                            *is_storage_valid = false;
                        }
                    }
                    Err(_) => {
                        warn!(
                            "Can't get the block number for tx {}! The block locator entry for hash {} is missing",
                            hex::encode(tx_id),
                            BlockHeaderHash(tx_location.block_hash)
                        );

                        if let Some(ref mut fix) = database_fix {
                            let corrected_location = TransactionLocation {
                                index: block_tx_idx as u32,
                                block_hash: block.header.get_hash().0,
                            };

                            match to_bytes!(corrected_location) {
                                Ok(location_bytes) => {
                                    fix.push(Op::Insert {
                                        col: COL_TRANSACTION_LOCATION,
                                        key: tx_id.to_vec(),
                                        value: location_bytes,
                                    });
                                }
                                Err(e) => {
                                    error!("Can't create a block locator fix for tx {}: {}", hex::encode(tx_id), e);
                                    *is_storage_valid = false;
                                }
                            }
                        } else {
                            *is_storage_valid = false;
                        }
                    }
                },
                Err(e) => {
                    error!("Can't get the location of tx {}: {}", hex::encode(tx_id), e);
                    *is_storage_valid = false;
                }
                Ok(None) => {
                    error!("Can't get the location of tx {}", hex::encode(tx_id));
                    *is_storage_valid = false;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use snarkos_testing::sync::TestBlocks;

    #[tokio::test]
    async fn valid_storage_validates() {
        //tracing_subscriber::fmt::init();

        let consensus = snarkos_testing::sync::create_test_consensus();

        let blocks = TestBlocks::load(Some(10), "test_blocks_100_1").0;
        for block in blocks {
            consensus.receive_block(&block).await.unwrap();
        }

        assert!(consensus.ledger.validate(None, false));
    }
}
