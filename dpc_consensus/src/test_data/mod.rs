use crate::ConsensusParameters;

use snarkos_dpc::{
    address::{AddressPair, AddressPublicKey},
    base_dpc::{instantiated::*, predicate::DPCPredicate, record::DPCRecord},
    DPCScheme,
};

use snarkos_objects::{
    dpc::{transactions::DPCTransactions, Block},
    ledger::Ledger,
    merkle_root,
    BlockHeader,
    MerkleRootHash,
};

use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

pub const TEST_CONSENSUS: ConsensusParameters = ConsensusParameters {
    max_block_size: 1_000_000usize,
    max_nonce: u32::max_value(),
    target_block_time: 2i64, //unix seconds
};

pub fn create_block_with_coinbase_transaction<R: Rng>(
    transactions: &mut DPCTransactions<Tx>,
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    genesis_pred_vk_bytes: &Vec<u8>,
    new_birth_predicates: Vec<DPCPredicate<Components>>,
    new_death_predicates: Vec<DPCPredicate<Components>>,
    genesis_address: AddressPair<Components>,
    recipient: AddressPublicKey<Components>,
    consensus: &ConsensusParameters,
    ledger: &MerkleTreeLedger,
    rng: &mut R,
) -> (Vec<DPCRecord<Components>>, Block<Tx>) {
    let (new_coinbase_records, transaction) = ConsensusParameters::create_coinbase_transaction(
        ledger.len() as u32,
        &transactions,
        &parameters,
        &genesis_pred_vk_bytes,
        new_birth_predicates,
        new_death_predicates,
        genesis_address,
        recipient,
        &ledger,
        rng,
    )
    .unwrap();

    transactions.push(transaction);

    let transaction_ids: Vec<Vec<u8>> = transactions
        .to_transaction_ids()
        .unwrap()
        .iter()
        .map(|id| id.to_vec())
        .collect();

    let mut merkle_root_bytes = [0u8; 32];
    merkle_root_bytes[..].copy_from_slice(&merkle_root(&transaction_ids));

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64;

    let previous_block = ledger.get_latest_block().unwrap();

    // Pseudo mining
    let header = BlockHeader {
        previous_block_hash: previous_block.header.get_hash(),
        merkle_root_hash: MerkleRootHash(merkle_root_bytes),
        time,
        difficulty_target: consensus.get_block_difficulty(&previous_block.header, time),
        nonce: 0, // TODO integrate with actual miner nonce generation
    };

    let block = Block {
        header,
        transactions: transactions.clone(),
    };

    (new_coinbase_records, block)
}
