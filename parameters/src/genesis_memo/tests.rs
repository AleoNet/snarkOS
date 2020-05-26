use crate::genesis_memo::GenesisMemo;
use snarkos_models::parameters::Parameter;

#[test]
fn test_genesis_memo() {
    let parameters = GenesisMemo::load_bytes();
    assert_eq!(GenesisMemo::SIZE, parameters.len() as u64);
}
