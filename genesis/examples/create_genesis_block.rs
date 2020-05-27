//// TODO (raychu86) implement finding the genesis block from the stored transactions in `/src/transaction_*`
//
//use snarkos_dpc::base_dpc::{instantiated::Components, transaction::DPCTransaction, BaseDPCComponents};
//use snarkos_objects::{merkle_root, BlockHeader, BlockHeaderHash, DPCTransactions, MerkleRootHash};
//use snarkos_consensus::{BlockHeader, BlockHeaderHash, ConsensusParameters, DPCTransactions, MerkleRootHash};
//use snarkos_errors::consensus::ConsensusError;
//use snarkos_genesis::Transaction1;
//use snarkos_utilities::bytes::{}
//
//use std::{
//    fs::File,
//    io::{Result as IoResult, Write},
//    path::PathBuf,
//};
//
//pub fn generate<C: BaseDPCComponents>() -> Result<Vec<u8>, ConsensusError> {
//    let consensus = ConsensusParameters {
//        max_block_size: 1_000_000_000usize,
//        max_nonce: u32::max_value(),
//        target_block_time: 10i64,
//    };
//
//
//    let previous_block_header = BlockHeader {
//        previous_block_hash: BlockHeaderHash([0u8; 32]),
//        merkle_root_hash: MerkleRootHash([0u8; 32]),
//        time: 0,
//        difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
//        nonce: 0,
//    };
//
//    let transactions = DPCTransaction::<C>::read()
//
//    let mut merkle_root_bytes = [0u8; 32];
//    merkle_root_bytes[..].copy_from_slice(&merkle_root(&transactions.to_transaction_ids()?));
//
//    let time = Utc::now().timestamp();
//
//    Ok(block_header.serialize().to_vec())
//}
//
//pub fn store(path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
//    let mut file = File::create(path)?;
//    file.write_all(&bytes)?;
//    drop(file);
//    Ok(())
//}
//
//pub fn main() {
//    let bytes = generate::<Components>().unwrap();
//    let filename = PathBuf::from("block_header.genesis");
//    store(&filename, &bytes).unwrap();
//}
