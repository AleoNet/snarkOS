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

use snarkos_consensus::error::ConsensusError;
use snarkos_consensus::ConsensusParameters;
use snarkos_consensus::MemoryPool;
use snarkos_consensus::MerkleTreeLedger;
use snarkos_consensus::Miner;
use snarkos_testing::consensus::*;
use snarkvm_dpc::base_dpc::instantiated::*;
use snarkvm_dpc::base_dpc::record::DPCRecord;
use snarkvm_dpc::base_dpc::record_payload::RecordPayload;
use snarkvm_models::dpc::DPCScheme;
use snarkvm_models::dpc::Program;
use snarkvm_models::dpc::Record;
use snarkvm_objects::dpc::DPCTransactions;
use snarkvm_objects::Account;
use snarkvm_objects::AccountAddress;
use snarkvm_objects::Block;
use snarkvm_utilities::bytes::ToBytes;

use rand::Rng;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

fn setup_test_data() -> Result<TestData, ConsensusError> {
    // get the params
    let parameters = &FIXTURE.parameters;
    let ledger = FIXTURE.ledger();
    let [miner_acc, acc_1, _] = FIXTURE.test_accounts.clone();
    let mut rng = FIXTURE.rng.clone();
    let consensus = Arc::new(TEST_CONSENSUS.clone());

    // setup the miner
    let miner = Miner::new(miner_acc.address.clone(), consensus.clone());
    let mut memory_pool = MemoryPool::new();

    // mine an empty block
    let (block_1, coinbase_records) = mine_block(&miner, &ledger, &parameters, &consensus, &mut memory_pool, vec![])?;

    // make a tx which spends 10 to the BaseDPCComponents receiver
    let (_records_1, tx_1) = send(
        &ledger,
        &parameters,
        &consensus,
        &miner_acc,
        coinbase_records.clone(),
        &acc_1.address,
        10,
        &mut rng,
    )?;

    // mine the block
    let (block_2, coinbase_records_2) =
        mine_block(&miner, &ledger, &parameters, &consensus, &mut memory_pool, vec![tx_1])?;

    // Find alternative conflicting/late blocks

    let alternative_block_1_header = miner.find_block(
        &block_1.transactions,
        &ledger.get_block_header(&block_1.header.previous_block_hash)?,
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

fn mine_block(
    miner: &Miner,
    ledger: &MerkleTreeLedger,
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::NetworkParameters,
    consensus: &ConsensusParameters,
    memory_pool: &mut MemoryPool<Tx>,
    txs: Vec<Tx>,
) -> Result<(Block<Tx>, Vec<DPCRecord<Components>>), ConsensusError> {
    let transactions = DPCTransactions(txs);

    let (previous_block_header, transactions, coinbase_records) =
        miner.establish_block(&parameters, ledger, &transactions)?;

    let header = miner.find_block(&transactions, &previous_block_header)?;

    let block = Block { header, transactions };

    let old_block_height = ledger.get_current_block_height();

    // add it to the chain
    consensus.receive_block(&parameters, ledger, memory_pool, &block)?;

    let new_block_height = ledger.get_current_block_height();
    assert_eq!(old_block_height + 1, new_block_height);

    // Duplicate blocks dont do anything
    consensus.receive_block(&parameters, ledger, memory_pool, &block)?;

    let new_block_height = ledger.get_current_block_height();
    assert_eq!(old_block_height + 1, new_block_height);

    Ok((block, coinbase_records))
}

/// Spends some value from inputs owned by the sender, to the receiver,
/// and pays back whatever we are left with.
#[allow(clippy::too_many_arguments)]
fn send<R: Rng>(
    ledger: &MerkleTreeLedger,
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::NetworkParameters,
    consensus: &ConsensusParameters,
    from: &Account<Components>,
    inputs: Vec<DPCRecord<Components>>,
    receiver: &AccountAddress<Components>,
    amount: u64,
    rng: &mut R,
) -> Result<(Vec<DPCRecord<Components>>, Tx), ConsensusError> {
    let mut sum = 0;
    for inp in &inputs {
        sum += inp.value();
    }
    assert!(sum >= amount, "not enough balance in inputs");
    let change = sum - amount;

    let input_programs = vec![FIXTURE.program.into_compact_repr(); NUM_INPUT_RECORDS];
    let output_programs = vec![FIXTURE.program.into_compact_repr(); NUM_OUTPUT_RECORDS];

    let to = vec![receiver.clone(), from.address.clone()];
    let values = vec![amount, change];
    let output = vec![RecordPayload::default(); NUM_OUTPUT_RECORDS];
    let dummy_flags = vec![false; NUM_OUTPUT_RECORDS];

    let from = vec![from.private_key.clone(); NUM_INPUT_RECORDS];
    consensus.create_transaction(
        parameters,
        inputs,
        from,
        to,
        input_programs,
        output_programs,
        dummy_flags,
        values,
        output,
        [0u8; 32],
        &ledger,
        rng,
    )
}

pub fn main() {
    let test_data = setup_test_data().unwrap();

    const TEST_DATA_FILE: &str = "test_data";

    let file = std::io::BufWriter::new(File::create(PathBuf::from(TEST_DATA_FILE)).expect("could not open file"));
    test_data.write(file).expect("could not write to file");
}
