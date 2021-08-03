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

#[macro_use]
extern crate tracing;

use snarkos_consensus::{error::ConsensusError, Consensus, CreateTransactionRequest, MineContext, TransactionResponse};
use snarkos_storage::{PrivateKey, SerialBlock, SerialBlockHeader, SerialRecord, SerialTransaction};
use snarkos_testing::sync::*;
use snarkvm_dpc::{
    testnet1::{instantiated::*, record::payload::Payload as RecordPayload},
    Account,
    Address,
    AleoAmount,
    DPCComponents,
};
use tracing_subscriber::EnvFilter;

use rand::{CryptoRng, Rng};
use std::{fs::File, path::PathBuf};

async fn mine_block(
    miner: &MineContext,
    transactions: Vec<SerialTransaction>,
    parent_block_header: &SerialBlockHeader,
) -> Result<(SerialBlock, Vec<SerialRecord>), ConsensusError> {
    info!("Mining block!");

    let (transactions, coinbase_records) = miner.establish_block(transactions).await?;

    let header = miner.find_block(&transactions, parent_block_header)?;

    let block = SerialBlock { header, transactions };

    let old_block_height = miner.consensus.storage.canon().await?.block_height;

    // Duplicate blocks dont do anything
    miner.consensus.receive_block(block.clone()).await; // throws a duplicate error -- seemingly intentional

    let new_block_height = miner.consensus.storage.canon().await?.block_height;
    assert_eq!(old_block_height + 1, new_block_height);

    Ok((block, coinbase_records))
}

/// Spends some value from inputs owned by the sender, to the receiver,
/// and pays back whatever we are left with.
#[allow(clippy::too_many_arguments)]
async fn send<R: Rng + CryptoRng>(
    consensus: &Consensus,
    from: &Account<Components>,
    inputs: Vec<SerialRecord>,
    receiver: &Address<Components>,
    amount: i64,
    rng: &mut R,
    memo: [u8; 32],
) -> Result<TransactionResponse, ConsensusError> {
    let mut sum = 0;
    for inp in &inputs {
        sum += inp.value.0;
    }
    assert!(sum >= amount, "not enough balance in inputs");
    let change = sum - amount;

    let to = vec![receiver.clone(), from.address.clone()];
    let values = vec![amount, change];

    let from: Vec<PrivateKey> = vec![from.private_key.clone(); Components::NUM_INPUT_RECORDS]
        .into_iter()
        .map(Into::into)
        .collect();

    let joint_serial_numbers = consensus.calculate_joint_serial_numbers(&inputs[..], &from[..])?;
    let mut new_records = vec![];
    for j in 0..Components::NUM_OUTPUT_RECORDS as u8 {
        new_records.push(consensus.make_dummy_record(
            &joint_serial_numbers[..],
            j + Components::NUM_INPUT_RECORDS as u8,
            to[j as usize].clone().into(),
            AleoAmount(values[j as usize]),
            RecordPayload::default(),
        )?);
    }
    consensus
        .create_transaction(CreateTransactionRequest {
            old_records: inputs,
            old_account_private_keys: from,
            new_records,
            memo,
        })
        .await
}

async fn mine_blocks(n: u32) -> Result<TestBlocks, ConsensusError> {
    info!("Creating test account");
    let [miner_acc, acc_1, _] = FIXTURE.test_accounts.clone();
    let mut rng = FIXTURE.rng.clone();
    info!("Creating sync");
    let consensus = crate::create_test_consensus().await;

    // setup the miner
    info!("Creating miner");
    let miner = MineContext::prepare(miner_acc.address.clone(), consensus.clone()).await?;
    info!("Creating memory pool");

    let mut txs = vec![];
    let mut blocks = vec![];
    let last_block_header = consensus.storage.get_block_hash(0).await?.unwrap();
    let mut last_block_header = consensus.storage.get_block_header(&last_block_header).await?;

    for i in 0..n {
        // mine an empty block
        let (block, coinbase_records) = mine_block(&miner, txs.clone(), &last_block_header).await?;

        txs.clear();
        let mut memo = [0u8; 32];
        memo[0] = i as u8;
        // make a tx which spends 10 to the Testnet1Components receiver
        let response: TransactionResponse = send(
            &consensus,
            &miner_acc,
            coinbase_records.clone(),
            &acc_1.address,
            (10 + i).into(),
            &mut rng,
            memo,
        )
        .await?;

        txs.push(response.transaction);
        last_block_header = block.header.clone();
        blocks.push(block);
    }

    Ok(TestBlocks::new(blocks))
}

#[tokio::main]
pub async fn main() {
    let filter = EnvFilter::from_default_env();
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    let block_count = 100;

    info!("Setting up test data");
    let test_blocks = mine_blocks(block_count).await.unwrap();

    let file = std::io::BufWriter::new(
        File::create(PathBuf::from(format!("test_blocks_{}", block_count))).expect("could not open file"),
    );
    test_blocks.write_le(file).expect("could not write to file");
}
