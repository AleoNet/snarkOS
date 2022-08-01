// Copyright (C) 2019-2022 Aleo Systems Inc.
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
    storage::{rocksdb::RocksDB, ReadWrite, Storage},
    LedgerState,
};
use rayon::iter::ParallelIterator;
use snarkos_environment::CurrentNetwork;
use snarkvm::dpc::{prelude::*, testnet2::Testnet2};

use rand::{thread_rng, Rng};
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

fn temp_dir() -> std::path::PathBuf {
    tempfile::tempdir().expect("Failed to open temporary directory").into_path()
}

/// Returns 3 test blocks.
// Note: the `blocks_3` file was generated on a testnet2 storage using `LedgerState::dump_blocks`.
fn test_blocks_3() -> Vec<Block<CurrentNetwork>> {
    let mut test_block_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_block_path.push("benches");
    test_block_path.push("blocks_3");

    let test_blocks = fs::read(test_block_path).unwrap_or_else(|_| panic!("Missing the test blocks file"));
    bincode::deserialize(&test_blocks).expect("Failed to deserialize a block dump")
}

/// Initializes a new instance of the ledger.
fn create_new_ledger<N: Network, S: Storage<Access = ReadWrite>>() -> LedgerState<N, ReadWrite> {
    LedgerState::open_writer_with_increment::<S, _>(temp_dir(), 1).expect("Failed to initialize ledger")
}

#[test]
fn test_genesis() {
    // Initialize a new ledger.
    let ledger = create_new_ledger::<CurrentNetwork, RocksDB>();

    // Retrieve the genesis block.
    let genesis = CurrentNetwork::genesis_block();

    // Initialize a new ledger tree.
    let mut ledger_tree = LedgerTree::<CurrentNetwork>::new().expect("Failed to initialize ledger tree");
    ledger_tree.add(&genesis.hash()).expect("Failed to add to ledger tree");

    // Ensure the ledger is at the genesis block.
    assert_eq!(0, ledger.latest_block_height());
    assert_eq!(genesis.height(), ledger.latest_block_height());
    assert_eq!(genesis.hash(), ledger.latest_block_hash());
    assert_eq!(genesis.timestamp(), ledger.latest_block_timestamp());
    assert_eq!(genesis.difficulty_target(), ledger.latest_block_difficulty_target());
    assert_eq!(genesis, &ledger.latest_block());
    assert_eq!(Some(&(genesis.hash(), None)), ledger.latest_block_locators().get(&genesis.height()));
    assert_eq!(ledger_tree.root(), ledger.latest_ledger_root());
}

#[test]
fn test_add_next_block() {
    // Initialize a new ledger.
    let ledger = create_new_ledger::<CurrentNetwork, RocksDB>();
    assert_eq!(0, ledger.latest_block_height());

    // Initialize a new ledger tree.
    let mut ledger_tree = LedgerTree::<CurrentNetwork>::new().expect("Failed to initialize ledger tree");
    ledger_tree
        .add(&CurrentNetwork::genesis_block().hash())
        .expect("Failed to add to ledger tree");

    // Load a test block.
    let block = test_blocks_3().remove(0);
    ledger.add_next_block(&block).expect("Failed to add next block to ledger");
    ledger_tree.add(&block.hash()).expect("Failed to add hash to ledger tree");

    // Ensure the ledger is at block 1.
    assert_eq!(1, ledger.latest_block_height());
    assert_eq!(block.height(), ledger.latest_block_height());
    assert_eq!(block.hash(), ledger.latest_block_hash());
    assert_eq!(block.timestamp(), ledger.latest_block_timestamp());
    assert_eq!(block.difficulty_target(), ledger.latest_block_difficulty_target());
    assert_eq!(block, ledger.latest_block());
    assert_eq!(ledger_tree.root(), ledger.latest_ledger_root());

    // Retrieve the genesis block.
    let genesis = CurrentNetwork::genesis_block();

    // Ensure the block locators are correct.
    let block_locators = ledger.latest_block_locators();
    assert_eq!(2, block_locators.len());
    assert_eq!(
        Some(&(block.hash(), Some(block.header().clone()))),
        block_locators.get(&block.height())
    );
    assert_eq!(Some(&(genesis.hash(), None)), block_locators.get(&genesis.height()));
}

#[test]
fn test_remove_last_block() {
    // Initialize a new ledger.
    let ledger = create_new_ledger::<CurrentNetwork, RocksDB>();
    assert_eq!(0, ledger.latest_block_height());

    // Initialize a new ledger tree.
    let mut ledger_tree = LedgerTree::<CurrentNetwork>::new().expect("Failed to initialize ledger tree");
    ledger_tree
        .add(&CurrentNetwork::genesis_block().hash())
        .expect("Failed to add to ledger tree");

    // Load a test block.
    let block = test_blocks_3().remove(0);
    ledger.add_next_block(&block).expect("Failed to add next block to ledger");
    assert_eq!(1, ledger.latest_block_height());

    // Remove the last block.
    let blocks = ledger
        .revert_to_block_height(ledger.latest_block_height() - 1)
        .expect("Failed to remove the last block");
    assert_eq!(vec![block], blocks);

    // Retrieve the genesis block.
    let genesis = CurrentNetwork::genesis_block();

    // Ensure the ledger is back at the genesis block.
    assert_eq!(0, ledger.latest_block_height());
    assert_eq!(genesis.height(), ledger.latest_block_height());
    assert_eq!(genesis.hash(), ledger.latest_block_hash());
    assert_eq!(genesis.timestamp(), ledger.latest_block_timestamp());
    assert_eq!(genesis.difficulty_target(), ledger.latest_block_difficulty_target());
    assert_eq!(genesis, &ledger.latest_block());
    assert_eq!(Some(&(genesis.hash(), None)), ledger.latest_block_locators().get(&genesis.height()));
    assert_eq!(ledger_tree.root(), ledger.latest_ledger_root());
}

#[test]
fn test_remove_last_2_blocks() {
    // Initialize a new ledger.
    let ledger = create_new_ledger::<CurrentNetwork, RocksDB>();
    assert_eq!(0, ledger.latest_block_height());

    // Initialize a new ledger tree.
    let mut ledger_tree = LedgerTree::<CurrentNetwork>::new().expect("Failed to initialize ledger tree");
    ledger_tree
        .add(&CurrentNetwork::genesis_block().hash())
        .expect("Failed to add to ledger tree");

    // Load test blocks.
    let mut test_blocks = test_blocks_3();
    let _block_3 = test_blocks.pop().unwrap();
    let block_2 = test_blocks.pop().unwrap();
    let block_1 = test_blocks.pop().unwrap();

    ledger.add_next_block(&block_1).expect("Failed to add next block to ledger");
    ledger.add_next_block(&block_2).expect("Failed to add next block to ledger");

    // Remove the last block.
    let blocks = ledger
        .revert_to_block_height(ledger.latest_block_height() - 2)
        .expect("Failed to remove the last two blocks");
    assert_eq!(vec![block_1, block_2], blocks);

    // Retrieve the genesis block.
    let genesis = CurrentNetwork::genesis_block();

    // Ensure the ledger is back at the genesis block.
    assert_eq!(0, ledger.latest_block_height());
    assert_eq!(genesis.height(), ledger.latest_block_height());
    assert_eq!(genesis.hash(), ledger.latest_block_hash());
    assert_eq!(genesis.timestamp(), ledger.latest_block_timestamp());
    assert_eq!(genesis.difficulty_target(), ledger.latest_block_difficulty_target());
    assert_eq!(genesis, &ledger.latest_block());
    assert_eq!(Some(&(genesis.hash(), None)), ledger.latest_block_locators().get(&genesis.height()));
    assert_eq!(ledger_tree.root(), ledger.latest_ledger_root());
}

#[test]
fn test_get_block_locators() {
    // Initialize a new ledger.
    let ledger = create_new_ledger::<CurrentNetwork, RocksDB>();
    assert_eq!(0, ledger.latest_block_height());

    // Initialize a new ledger tree.
    let mut ledger_tree = LedgerTree::<CurrentNetwork>::new().expect("Failed to initialize ledger tree");
    ledger_tree
        .add(&CurrentNetwork::genesis_block().hash())
        .expect("Failed to add to ledger tree");

    // Load test blocks.
    let test_blocks = test_blocks_3();

    // Check the block locators after each block insertion.
    for block in test_blocks {
        ledger.add_next_block(&block).expect("Failed to add next block to ledger");
        let block_locators = ledger
            .get_block_locators(ledger.latest_block_height())
            .expect("Failed to get block locators");
        assert!(
            ledger
                .check_block_locators(&block_locators)
                .expect("Failed to check block locators")
        );
    }
}

#[test]
fn test_transaction_fees() {
    let rng = &mut thread_rng();
    let terminator = AtomicBool::new(false);

    // Initialize a new ledger.
    let ledger = create_new_ledger::<CurrentNetwork, RocksDB>();
    assert_eq!(0, ledger.latest_block_height());

    // Initialize a new account.
    let account = Account::<CurrentNetwork>::new(&mut thread_rng());
    let private_key = account.private_key();
    let view_key = account.view_key();
    let address = account.address();

    // Mine the next block.
    let (block, _record) = ledger
        .mine_next_block(address, true, &[], &terminator, rng)
        .expect("Failed to mine");
    ledger.add_next_block(&block).expect("Failed to add next block to ledger");

    // Craft the transaction variables.
    let coinbase_transaction = &block.transactions()[0];

    let available_balance = AleoAmount::from_i64(-coinbase_transaction.value_balance().0);
    let fee = AleoAmount::from_i64(rng.gen_range(1..available_balance.0));
    let amount = available_balance.sub(fee);
    let coinbase_record = coinbase_transaction.to_decrypted_records(&view_key.into()).collect::<Vec<_>>();

    let ledger_proof = ledger.get_ledger_inclusion_proof(coinbase_record[0].commitment()).unwrap();

    // Initialize a recipient account.
    let recipient_account = Account::<CurrentNetwork>::new(rng);
    let recipient_view_key = recipient_account.view_key();
    let recipient = recipient_account.address();

    // Craft the transaction with a random fee.
    let transfer_request = Request::new_transfer(
        private_key,
        coinbase_record,
        vec![ledger_proof, LedgerProof::default()],
        recipient,
        amount,
        fee,
        true,
        rng,
    )
    .unwrap();

    let (vm, _response) = VirtualMachine::new(ledger.latest_ledger_root())
        .unwrap()
        .execute(&transfer_request, rng)
        .unwrap();

    let new_transaction = vm.finalize().unwrap();

    // Mine the next block.
    let (block_2, _record) = ledger
        .mine_next_block(address, true, &[new_transaction], &terminator, rng)
        .expect("Failed to mine");
    ledger.add_next_block(&block_2).expect("Failed to add next block to ledger");
    assert_eq!(2, ledger.latest_block_height());

    let expected_block_reward = Block::<CurrentNetwork>::block_reward(2).add(fee);
    let output_record = &block_2.transactions()[0]
        .to_decrypted_records(&recipient_view_key.into())
        .collect::<Vec<_>>()[0];
    let new_coinbase_record = &block_2.transactions()[1].to_decrypted_records(&view_key.into()).collect::<Vec<_>>()[0];

    // Check that the output record balances are correct.
    assert_eq!(new_coinbase_record.value(), expected_block_reward);
    assert_eq!(output_record.value(), amount);
}

#[test]
fn test_get_blocks_iterator() {
    // Initialize a new temporary directory.
    let directory = temp_dir();

    // Initialize an empty ledger.
    let ledger_state = LedgerState::open_writer::<RocksDB, _>(directory.clone()).expect("Failed to initialize ledger");

    // Read the test blocks;
    // note: they don't include the genesis block, as it's always available when creating a ledger.
    let test_blocks = fs::read("benches/blocks_1").expect("Missing the test blocks file");
    let blocks: Vec<Block<Testnet2>> = bincode::deserialize(&test_blocks).expect("Failed to deserialize a block dump");

    // Load a test block into the ledger.
    ledger_state.add_next_block(&blocks[0]).expect("Failed to add a test block");
    let blocks_result: Vec<_> = ledger_state
        .get_blocks(0, ledger_state.latest_block_height() + 1)
        .unwrap()
        .filter_map(|block_result| block_result.ok())
        .collect();

    drop(ledger_state);

    assert_eq!(blocks_result, vec![Testnet2::genesis_block().clone(), blocks[0].clone()]);
}
