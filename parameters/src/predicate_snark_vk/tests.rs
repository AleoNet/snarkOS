use crate::predicate_snark_vk::PredicateSNARKVKParameters;
use snarkos_models::parameters::Parameter;

#[test]
fn test_predicate_snark_vk_parameters() {
    let parameters = PredicateSNARKVKParameters::load_bytes();
    assert_eq!(PredicateSNARKVKParameters::SIZE, parameters.len() as u64);
}
