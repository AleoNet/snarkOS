// TODO (raychu86) implement finding the genesis block from the stored transactions in `/src/transaction_*`

use snarkos_objects::{BlockHeader, BlockHeaderHash, MerkleRootHash};
use std::{
    fs::File,
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn generate() -> Vec<u8> {
    let block_header = BlockHeader {
        previous_block_hash: BlockHeaderHash([0u8; 32]),
        merkle_root_hash: MerkleRootHash([0u8; 32]),
        time: 0,
        difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
        nonce: 0,
    };

    block_header.serialize().to_vec()
}

pub fn store(path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let bytes = generate();
    let filename = PathBuf::from("block_header.genesis");
    store(&filename, &bytes).unwrap();
}
