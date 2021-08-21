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

use std::sync::atomic::AtomicBool;

use snarkos_consensus::{error::ConsensusError, Consensus, CreateTransactionRequest, MineContext, TransactionResponse};
use snarkos_storage::{PrivateKey, SerialBlock, SerialBlockHeader, SerialRecord, SerialTransaction};
use snarkvm_dpc::{
    testnet1::{instantiated::*, record::payload::Payload as RecordPayload},
    Account,
    Address,
    AleoAmount,
    DPCComponents,
};

pub async fn mine_block(
    miner: &MineContext,
    transactions: Vec<SerialTransaction>,
    parent_block_header: &SerialBlockHeader,
) -> Result<(SerialBlock, Vec<SerialRecord>), ConsensusError> {
    let (transactions, coinbase_records) = miner.establish_block(transactions).await?;

    let header = miner.find_block(&transactions, parent_block_header, &AtomicBool::new(false))?;

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
pub async fn send(
    consensus: &Consensus,
    from: &Account<Components>,
    inputs: Vec<SerialRecord>,
    receiver: &Address<Components>,
    amount: i64,
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
            j,
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
