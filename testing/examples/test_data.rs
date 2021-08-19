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

use snarkos_consensus::{MineContext, TransactionResponse};
use snarkos_testing::{
    mining::{mine_block, send},
    sync::*,
};

use snarkvm_utilities::ToBytes;
use std::{fs::File, path::PathBuf, sync::atomic::AtomicBool};

async fn setup_test_data() -> TestData {
    let [miner_acc, acc_1, _] = FIXTURE.test_accounts.clone();
    let consensus = snarkos_testing::sync::create_test_consensus().await;

    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // setup the miner
    let miner = MineContext::prepare(miner_acc.address.clone(), consensus.clone())
        .await
        .unwrap();

    let canon = consensus.storage.canon().await.unwrap();
    let header = consensus.storage.get_block_header(&canon.hash).await.unwrap();

    // mine an empty block
    let (block_1, coinbase_records) = mine_block(&miner, vec![], &header).await.unwrap();

    // make a tx which spends 10 to the Testnet1Components receiver
    let response: TransactionResponse = send(
        &consensus,
        &miner_acc,
        coinbase_records.clone(),
        &acc_1.address,
        10,
        [0u8; 32],
    )
    .await
    .unwrap();

    // mine the block
    let (block_2, coinbase_records_2) = mine_block(&miner, vec![response.transaction], &block_1.header)
        .await
        .unwrap();

    // Find alternative conflicting/late blocks

    let alternative_block_1_header = miner
        .find_block(
            &block_1.transactions,
            &consensus
                .storage
                .get_block_header(&block_1.header.previous_block_hash)
                .await
                .unwrap(),
            &AtomicBool::new(false),
        )
        .unwrap();
    let alternative_block_2_header = miner
        .find_block(
            &block_2.transactions,
            &alternative_block_1_header,
            &AtomicBool::new(false),
        )
        .unwrap();

    TestData {
        block_1,
        block_2,
        records_1: coinbase_records,
        records_2: coinbase_records_2,
        alternative_block_1_header,
        alternative_block_2_header,
    }
}

#[tokio::main]
pub async fn main() {
    let test_data = setup_test_data().await;

    const TEST_DATA_FILE: &str = "test_data";

    let file = std::io::BufWriter::new(File::create(PathBuf::from(TEST_DATA_FILE)).expect("could not open file"));
    test_data.write_le(file).expect("could not write to file");
}
