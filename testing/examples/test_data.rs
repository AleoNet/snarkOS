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

use snarkos_consensus::{error::ConsensusError, Consensus, Miner};
use snarkos_testing::sync::*;
use snarkvm::{
    dpc::{payload::Payload as RecordPayload, testnet1::*, Account, Address, Parameters, Record, RecordScheme},
    ledger::{Block, LedgerScheme, Storage, Transactions},
    utilities::{to_bytes_le, ToBytes},
};

use rand::{CryptoRng, Rng};
use std::{fs::File, path::PathBuf, sync::Arc};
//
// fn mine_block<S: Storage>(
//     miner: &Miner<S>,
//     txs: Vec<Testnet1Transaction>,
// ) -> Result<(Block<Testnet1Transaction>, Vec<Record<Testnet1Parameters>>), ConsensusError> {
//     let transactions = Transactions(txs);
//
//     let (previous_block_header, transactions, coinbase_records) = miner.establish_block(&transactions)?;
//
//     let header = miner.find_block(&transactions, &previous_block_header)?;
//
//     let block = Block { header, transactions };
//
//     let old_block_height = miner.consensus.ledger.block_height();
//
//     // add it to the chain
//     futures::executor::block_on(miner.consensus.receive_block(&block, false))?;
//
//     let new_block_height = miner.consensus.ledger.block_height();
//     assert_eq!(old_block_height + 1, new_block_height);
//
//     Ok((block, coinbase_records))
// }

async fn mine_block<S: Storage>(
    miner: &Miner<S>,
    txs: Vec<Testnet1Transaction>,
) -> Result<(Block<Testnet1Transaction>, Vec<Record<Testnet1Parameters>>), ConsensusError> {
    // Mine a new block.
    println!("Starting mining...");
    let (previous_block_header, transactions, coinbase_records) = miner.establish_block(&Transactions(txs))?;
    let header = miner.find_block(&transactions, &previous_block_header)?;
    let block = Block { header, transactions };

    // Duplicate blocks dont do anything
    let old_block_height = miner.consensus.ledger.block_height();
    miner.consensus.receive_block(&block, false).await.ok(); // throws a duplicate error -- seemingly intentional
    let new_block_height = miner.consensus.ledger.block_height();
    assert_eq!(old_block_height + 1, new_block_height);
    println!("Mined block {}", new_block_height);

    Ok((block, coinbase_records))
}

/// Spends some value from inputs owned by the sender, to the receiver,
/// and pays back whatever we are left with.
#[allow(clippy::too_many_arguments)]
fn create_send_transaction<R: Rng + CryptoRng, S: Storage>(
    consensus: &Consensus<S>,
    from: &Account<Testnet1Parameters>,
    inputs: Vec<Record<Testnet1Parameters>>,
    receiver: &Address<Testnet1Parameters>,
    amount: u64,
    rng: &mut R,
) -> Result<Testnet1Transaction, ConsensusError> {
    let mut sum = 0;
    for input in &inputs {
        sum += input.value();
    }
    assert!(sum >= amount, "not enough balance in inputs");
    let change = sum - amount;
    let values = vec![amount, change];
    let to = vec![receiver.clone(), from.address.clone()];

    let mut joint_serial_numbers = vec![];
    for i in 0..Testnet1Parameters::NUM_INPUT_RECORDS {
        let (sn, _) = inputs[i].to_serial_number(&from.private_key)?;
        joint_serial_numbers.extend_from_slice(&to_bytes_le![sn]?);
    }

    let mut new_records = vec![];
    for j in 0..Testnet1Parameters::NUM_OUTPUT_RECORDS {
        new_records.push(Record::new_full(
            &FIXTURE.program,
            to[j].clone(),
            false,
            values[j],
            RecordPayload::default(),
            (Testnet1Parameters::NUM_INPUT_RECORDS + j) as u8,
            joint_serial_numbers.clone(),
            rng,
        )?);
    }
    let from = vec![from.private_key.clone(); Testnet1Parameters::NUM_INPUT_RECORDS];

    consensus.create_transaction(inputs, from, new_records, None, rng)
}

async fn setup_test_data() -> Result<TestData, ConsensusError> {
    let mut rng = FIXTURE.rng.clone();

    // Load 2 accounts - 1 for the miner, and 1 for a receiver.
    let [miner_account, receiver_account, _] = FIXTURE.test_accounts.clone();

    // Initialize the miner.
    let consensus = Arc::new(snarkos_testing::sync::create_test_consensus());
    let miner = Miner::new(miner_account.address.clone(), consensus.clone());

    // Mine block 1 (empty).
    let (block_1, coinbase_records) = mine_block(&miner, vec![]).await?;

    // Create a transaction that sends 10 coins to the Testnet1Parameters receiver.
    let transaction = create_send_transaction(
        &consensus,
        &miner_account,
        coinbase_records.clone(),
        &receiver_account.address,
        10,
        &mut rng,
    )?;

    // Mine block 2 (with transaction).
    let (block_2, coinbase_records_2) = mine_block(&miner, vec![transaction]).await?;

    // Find alternative conflicting/late blocks
    let alternative_block_1_header = miner.find_block(
        &block_1.transactions,
        &consensus.ledger.get_block_header(&block_1.header.previous_block_hash)?,
    )?;
    let alternative_block_2_header = miner.find_block(&block_2.transactions, &alternative_block_1_header)?;

    let test_data = TestData {
        block_1,
        block_2,
        records_1: coinbase_records,
        records_2: coinbase_records_2,
        alternative_block_1_header,
        alternative_block_2_header,
    };

    Ok(test_data)
}

#[tokio::main]
pub async fn main() {
    const TEST_DATA_FILE: &str = "test_data";

    // Generate the test data.
    let test_data = setup_test_data().await.unwrap();

    // Store the test data to disk.
    let file = std::io::BufWriter::new(File::create(PathBuf::from(TEST_DATA_FILE)).expect("could not open file"));
    test_data.write_le(file).expect("could not write to file");
}
