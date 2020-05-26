use crate::genesis_account::GenesisAccount;
use snarkos_models::parameters::Parameter;

#[test]
fn test_genesis_account() {
    let parameters = GenesisAccount::load_bytes();
    assert_eq!(GenesisAccount::SIZE, parameters.len() as u64);
}
