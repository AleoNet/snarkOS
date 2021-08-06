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
    dpc::{testnet1::*, Account, Address, Parameters, Payload, Record, RecordScheme},
    ledger::{Block, LedgerScheme, Storage, Transactions},
    utilities::{to_bytes_le, ToBytes},
};

use rand::{CryptoRng, Rng};
use std::time::Instant;

pub async fn mine_block<S: Storage>(
    miner: &Miner<S>,
    txs: Vec<Testnet1Transaction>,
) -> Result<(Block<Testnet1Transaction>, Vec<Record<Testnet1Parameters>>), ConsensusError> {
    println!("Starting mining...");
    let timer = Instant::now();

    // Mine a new block.
    let (previous_block_header, transactions, coinbase_records) = miner.establish_block(&Transactions(txs))?;
    let header = miner.find_block(&transactions, &previous_block_header)?;
    let block = Block { header, transactions };

    // Duplicate blocks dont do anything
    let old_block_height = miner.consensus.ledger.block_height();
    miner.consensus.receive_block(&block, false).await.ok(); // throws a duplicate error -- seemingly intentional
    let new_block_height = miner.consensus.ledger.block_height();
    assert_eq!(old_block_height + 1, new_block_height);

    let elapsed = timer.elapsed().as_secs();
    println!("Mined block {} in {} seconds", new_block_height, elapsed);
    Ok((block, coinbase_records))
}

/// Spends some value from inputs owned by the sender, to the receiver,
/// and pays back whatever we are left with.
#[allow(clippy::too_many_arguments)]
pub fn create_send_transaction<R: Rng + CryptoRng, S: Storage>(
    consensus: &Consensus<S>,
    from: &Account<Testnet1Parameters>,
    inputs: Vec<Record<Testnet1Parameters>>,
    receiver: &Address<Testnet1Parameters>,
    amount: u64,
    rng: &mut R,
) -> Result<Testnet1Transaction, ConsensusError> {
    println!("Creating transaction...");
    let timer = Instant::now();

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
        new_records.push(Record::new_output(
            &FIXTURE.program,
            to[j].clone(),
            false,
            values[j],
            Payload::default(),
            (Testnet1Parameters::NUM_INPUT_RECORDS + j) as u8,
            joint_serial_numbers.clone(),
            rng,
        )?);
    }

    let from = vec![from.private_key.clone(); Testnet1Parameters::NUM_INPUT_RECORDS];

    let transaction = consensus.create_transaction(inputs, from, new_records, None, rng);
    println!("Created transaction in {} seconds", timer.elapsed().as_secs());

    transaction
}
