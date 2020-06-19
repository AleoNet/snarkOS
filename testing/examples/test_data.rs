use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger, Miner};
use snarkos_dpc::base_dpc::{instantiated::*, record::DPCRecord, record_payload::RecordPayload};
use snarkos_errors::consensus::ConsensusError;
use snarkos_models::{
    dpc::{DPCScheme, Record},
    objects::Transaction,
};
use snarkos_objects::{dpc::DPCTransactions, Account, AccountPublicKey, Block};
use snarkos_testing::consensus::*;
use snarkos_utilities::bytes::ToBytes;

use rand::Rng;
use std::{fs::File, path::PathBuf};

fn setup_test_data() -> Result<TestData, ConsensusError> {
    // get the params
    let parameters = &FIXTURE.parameters;
    let ledger = FIXTURE.ledger();
    let [miner_acc, acc_1, _] = FIXTURE.test_accounts.clone();
    let mut rng = FIXTURE.rng.clone();
    let consensus = TEST_CONSENSUS.clone();

    let network_id = FIXTURE.genesis_block.transactions[0].network_id();

    // setup the miner
    let miner = Miner::new(miner_acc.public_key.clone(), consensus.clone());
    let mut memory_pool = MemoryPool::new();

    // mine an empty block
    let (block_1, coinbase_records) = mine_block(
        &miner,
        &ledger,
        &parameters,
        &consensus,
        &mut memory_pool,
        vec![],
        network_id,
    )?;

    // make a tx which spends 10 to the BaseDPCComponents receiver
    let (_records_1, tx_1) = send(
        &ledger,
        &parameters,
        &miner_acc,
        coinbase_records.clone(),
        &acc_1.public_key,
        10,
        network_id,
        &mut rng,
    )?;

    // mine the block
    let (block_2, coinbase_records_2) = mine_block(
        &miner,
        &ledger,
        &parameters,
        &consensus,
        &mut memory_pool,
        vec![tx_1],
        network_id,
    )?;

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
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    consensus: &ConsensusParameters,
    memory_pool: &mut MemoryPool<Tx>,
    txs: Vec<Tx>,
    network_id: u8,
) -> Result<(Block<Tx>, Vec<DPCRecord<Components>>), ConsensusError> {
    let transactions = DPCTransactions(txs);

    let (previous_block_header, transactions, coinbase_records) =
        miner.establish_block(&parameters, ledger, &transactions, network_id)?;

    let header = miner.find_block(&transactions, &previous_block_header)?;

    let block = Block { header, transactions };

    let old_block_height = ledger.get_latest_block_height();

    // add it to the chain
    consensus.receive_block(&parameters, ledger, memory_pool, &block)?;

    let new_block_height = ledger.get_latest_block_height();
    assert_eq!(old_block_height + 1, new_block_height);

    // Duplicate blocks dont do anything
    consensus.receive_block(&parameters, ledger, memory_pool, &block)?;

    let new_block_height = ledger.get_latest_block_height();
    assert_eq!(old_block_height + 1, new_block_height);

    Ok((block, coinbase_records))
}

/// Spends some value from inputs owned by the sender, to the receiver,
/// and pays back whatever we are left with.
fn send<R: Rng>(
    ledger: &MerkleTreeLedger,
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    from: &Account<Components>,
    inputs: Vec<DPCRecord<Components>>,
    receiver: &AccountPublicKey<Components>,
    amount: u64,
    network_id: u8,
    rng: &mut R,
) -> Result<(Vec<DPCRecord<Components>>, Tx), ConsensusError> {
    let mut sum = 0;
    for inp in &inputs {
        sum += inp.value();
    }
    assert!(sum >= amount, "not enough balance in inputs");
    let change = sum - amount;

    let in_predicates = vec![FIXTURE.predicate.clone(); NUM_INPUT_RECORDS];
    let out_predicates = vec![FIXTURE.predicate.clone(); NUM_OUTPUT_RECORDS];

    let to = vec![receiver.clone(), from.public_key.clone()];
    let values = vec![amount, change];
    let output = vec![RecordPayload::default(); NUM_OUTPUT_RECORDS];
    let dummy_flags = vec![false; NUM_OUTPUT_RECORDS];

    let from = vec![from.private_key.clone(); NUM_INPUT_RECORDS];
    ConsensusParameters::create_transaction(
        parameters,
        inputs,
        from,
        to,
        in_predicates,
        out_predicates,
        dummy_flags,
        values,
        output,
        [0u8; 32], // TODO: Should we set these to anything?
        [0u8; 32],
        network_id,
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
