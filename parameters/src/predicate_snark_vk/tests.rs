use crate::predicate_snark_vk::PredicateSNARKVKParameters;
use snarkos_models::parameters::Parameters;

#[test]
fn test_predicate_snark_vk_parameters() {
    let parameters = PredicateSNARKVKParameters::load_bytes().expect("failed to load parameters");
    assert_eq!(PredicateSNARKVKParameters::SIZE, parameters.len() as u64);
}
