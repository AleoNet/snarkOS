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

use snarkos_consensus::{error::ConsensusError, Miner};
use snarkos_testing::sync::*;
use snarkvm::utilities::ToBytes;

use std::{fs::File, path::PathBuf, sync::Arc};

mod helpers;
use helpers::*;

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
