use crate::predicate_snark_pk::PredicateSNARKPKParameters;

#[test]
fn test_predicate_snark_pk_parameters() {
    let parameters = PredicateSNARKPKParameters::load_bytes();
    assert_eq!(8806582, parameters.len());
}
