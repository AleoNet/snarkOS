use snarkos_dpc::base_dpc::{instantiated::Components, transaction::DPCTransaction, BaseDPCComponents};
use snarkos_errors::objects::TransactionError;
use snarkos_genesis::Transaction1;
use snarkos_models::genesis::Genesis;
use snarkos_objects::{merkle_root, BlockHeader, BlockHeaderHash, DPCTransactions, MerkleRootHash};
use snarkos_utilities::bytes::FromBytes;

use chrono::Utc;
use std::{
    fs::File,
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn generate<C: BaseDPCComponents>() -> Result<Vec<u8>, TransactionError> {
    // Add transactions to block

    let mut transactions = DPCTransactions::new();

    let transaction_1 = DPCTransaction::<C>::read(Transaction1::load_bytes().as_slice())?;
    transactions.push(transaction_1);

    // Establish the merkle root hash of the transactions

    let mut merkle_root_bytes = [0u8; 32];
    merkle_root_bytes[..].copy_from_slice(&merkle_root(&transactions.to_transaction_ids()?));

    let genesis_header = BlockHeader {
        previous_block_hash: BlockHeaderHash([0u8; 32]),
        merkle_root_hash: MerkleRootHash(merkle_root_bytes),
        time: Utc::now().timestamp(),
        difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
        nonce: 0,
    };

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
