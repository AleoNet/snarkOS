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
    storage::{rocksdb::RocksDB, Storage},
    LedgerState,
};
use snarkvm::dpc::{prelude::*, testnet2::Testnet2};

use anyhow::{anyhow, Result};
use rand::thread_rng;
use std::sync::atomic::AtomicBool;

fn temp_dir() -> std::path::PathBuf {
    tempfile::tempdir().expect("Failed to open temporary directory").into_path()
}

/// Initializes a new instance of the ledger.
fn new_ledger<N: Network, S: Storage>() -> LedgerState<N> {
    LedgerState::open::<S, _>(temp_dir()).expect("Failed to initialize ledger")
}

/// Mines a new block using the latest state of the given ledger.
fn mine_next_block<N: Network>(ledger: &LedgerState<N>, recipient: Address<N>) -> Result<Block<N>> {
    // Prepare the new block.
    let previous_block_hash = ledger.latest_block_hash();
    let block_height = ledger.latest_block_height() + 1;

    // Compute the block difficulty target.
    let previous_timestamp = ledger.latest_block_timestamp()?;
    let previous_difficulty_target = ledger.latest_block_difficulty_target()?;
    let block_timestamp = chrono::Utc::now().timestamp();
    let difficulty_target = Blocks::<N>::compute_difficulty_target(previous_timestamp, previous_difficulty_target, block_timestamp);

    // Construct the ledger root.
    let ledger_root = ledger.latest_ledger_root();

    // Craft a coinbase transaction.
    let amount = Block::<N>::block_reward(block_height);
    let coinbase_transaction = Transaction::<N>::new_coinbase(recipient, amount, &mut thread_rng())?;

    // Construct the new block transactions.
    let transactions = Transactions::from(&[coinbase_transaction])?;

    // Mine the next block.
    match Block::mine(
        previous_block_hash,
        block_height,
        block_timestamp,
        difficulty_target,
        ledger_root,
        transactions,
        &AtomicBool::new(false),
        &mut thread_rng(),
    ) {
        Ok(block) => Ok(block),
        Err(error) => Err(anyhow!("Failed to mine the next block: {}", error)),
    }
}

#[test]
fn test_add_next_block() {
    // Initialize a new ledger.
    let mut ledger = new_ledger::<Testnet2, RocksDB>();
    // Initialize a new account.
    let account = Account::<Testnet2>::new(&mut thread_rng());

    assert_eq!(0, ledger.latest_block_height());

    let block = mine_next_block(&ledger, account.address()).expect("Failed to mine a block");
    ledger.add_next_block(&block).expect("Failed to add next block to ledger");

    assert_eq!(1, ledger.latest_block_height());
    assert_eq!(block.height(), ledger.latest_block_height());
    assert_eq!(block.block_hash(), ledger.latest_block_hash());
    assert_eq!(block.timestamp(), ledger.latest_block_timestamp().unwrap());
}
