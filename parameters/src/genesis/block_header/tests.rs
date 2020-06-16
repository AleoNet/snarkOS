use crate::block_header::GenesisBlockHeader;
use snarkos_models::genesis::Genesis;

#[test]
fn test_genesis_block_header() {
    let header = GenesisBlockHeader::load_bytes();
    assert_eq!(GenesisBlockHeader::SIZE, header.len() as u64);
}
