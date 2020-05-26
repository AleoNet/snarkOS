use crate::genesis_predicate_vk_bytes::GenesisPredicateVKBytes;
use snarkos_models::parameters::Parameter;

#[test]
fn test_genesis_predicate_vk_bytes() {
    let parameters = GenesisPredicateVKBytes::load_bytes();
    assert_eq!(GenesisPredicateVKBytes::SIZE, parameters.len() as u64);
}
