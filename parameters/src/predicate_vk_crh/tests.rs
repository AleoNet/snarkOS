use crate::predicate_vk_crh::PredicateVKCRHParameters;

#[test]
fn test_predicate_vk_crh_parameters() {
    let parameters = PredicateVKCRHParameters::load_bytes();
    assert_eq!(2188956, parameters.len());
}
