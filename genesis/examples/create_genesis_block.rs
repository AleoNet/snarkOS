use snarkos_consensus::ConsensusParameters;
use snarkos_dpc::base_dpc::{instantiated::Components, transaction::DPCTransaction, BaseDPCComponents};
use snarkos_errors::consensus::ConsensusError;
use snarkos_genesis::Transaction1;
use snarkos_models::genesis::Genesis;
use snarkos_objects::{merkle_root, BlockHeader, BlockHeaderHash, DPCTransactions, MerkleRootHash};
use snarkos_utilities::bytes::FromBytes;

use chrono::Utc;
use rand::{thread_rng, Rng};
use std::{
    fs::File,
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn mine_block(consensus: ConsensusParameters, unmined_header: BlockHeader) -> BlockHeader {
    let rng = &mut thread_rng();

    let mut hash_input = unmined_header.serialize();

    loop {
        let nonce = rng.gen_range(0, consensus.max_nonce);

        hash_input[80..84].copy_from_slice(&nonce.to_le_bytes());
        let hash_result = BlockHeader::deserialize(&hash_input).to_difficulty_hash();

        if hash_result <= unmined_header.difficulty_target {
            return BlockHeader::deserialize(&hash_input);
        }
    }
}

pub fn generate<C: BaseDPCComponents>() -> Result<Vec<u8>, ConsensusError> {
    let consensus = ConsensusParameters {
        max_block_size: 1_000_000_000usize,
        max_nonce: u32::max_value(),
        target_block_time: 10i64,
    };

    // Add transactions to block

    let mut transactions = DPCTransactions::new();

    let transaction_1 = DPCTransaction::<C>::read(Transaction1::load_bytes().as_slice())?;
    transactions.push(transaction_1);

    // Establish the merkle root hash of the transactions

    let mut merkle_root_bytes = [0u8; 32];
    merkle_root_bytes[..].copy_from_slice(&merkle_root(&transactions.to_transaction_ids()?));

    let unmined_header = BlockHeader {
        previous_block_hash: BlockHeaderHash([0u8; 32]),
        merkle_root_hash: MerkleRootHash(merkle_root_bytes),
        time: Utc::now().timestamp(),
        difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
        nonce: 0,
    };

    // Mine the block

    let genesis_header = mine_block(consensus, unmined_header);

    Ok(genesis_header.serialize().to_vec())
}

pub fn store(path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let bytes = generate::<Components>().unwrap();
    let filename = PathBuf::from("block_header.genesis");
    store(&filename, &bytes).unwrap();
}
