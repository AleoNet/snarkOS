use crate::GenesisBlock;
use snarkos_models::genesis::Genesis;

#[test]
fn test_genesis_block() {
    let block = GenesisBlock::load_bytes();
    assert_eq!(GenesisBlock::SIZE, block.len() as u64);
}
