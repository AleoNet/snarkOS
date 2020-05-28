use crate::predicate_vk_crh::PredicateVKCRHParameters;
use snarkos_models::parameters::Parameters;

#[test]
fn test_predicate_vk_crh_parameters() {
    let parameters = PredicateVKCRHParameters::load_bytes().expect("failed to load parameters");
    assert_eq!(PredicateVKCRHParameters::SIZE, parameters.len() as u64);
}
