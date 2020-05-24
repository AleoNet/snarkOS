use crate::predicate_snark_vk::PredicateSNARKVKParameters;

#[test]
fn test_predicate_snark_vk_parameters() {
    let parameters = PredicateSNARKVKParameters::load_bytes();
    assert_eq!(1359, parameters.len());
}
