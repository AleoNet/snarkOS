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

use std::{fs::File, path::PathBuf, sync::Arc};

mod helpers;
use helpers::*;

async fn mine_blocks(n: u32) -> Result<TestBlocks, ConsensusError> {
    let mut rng = FIXTURE.rng.clone();

    println!("Creating test accounts");
    let [miner_acc, acc_1, _] = FIXTURE.test_accounts.clone();
    println!("Created test account");

    println!("Creating sync");
    let consensus = Arc::new(crate::create_test_consensus());
    println!("Created sync");

    println!("Creating miner");
    let miner = Miner::new(miner_acc.address.clone(), consensus.clone());
    println!("Created miner");

    let mut transactions = vec![];
    let mut blocks = vec![];

    for i in 0..n {
        // Mine a block.
        let (block, coinbase_records) = mine_block(&miner, transactions.clone()).await?;

        // Create a transaction that sends 10 coins to the Testnet1Parameters receiver.
        transactions.clear();
        transactions.push(create_send_transaction(
            &consensus,
            &miner_acc,
            coinbase_records.clone(),
            &acc_1.address,
            (10 + i).into(),
            &mut rng,
        )?);

        blocks.push(block);
    }

    Ok(TestBlocks::new(blocks))
}

#[tokio::main]
pub async fn main() {
    let block_count = 100;
    let file = std::io::BufWriter::new(
        File::create(PathBuf::from(format!("test_blocks_{}", block_count))).expect("could not open file"),
    );

    println!("Mining {} blocks", block_count);
    let test_blocks = mine_blocks(block_count).await.unwrap();

    println!("Saving mined blocks to disk.");
    test_blocks.write_le(file).expect("could not write to file");
}
