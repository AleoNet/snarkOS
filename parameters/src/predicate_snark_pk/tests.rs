use crate::predicate_snark_pk::PredicateSNARKPKParameters;
use snarkos_models::parameters::Parameter;

#[test]
fn test_predicate_snark_pk_parameters() {
    let parameters = PredicateSNARKPKParameters::load_bytes();
    assert_eq!(PredicateSNARKPKParameters::SIZE, parameters.len() as u64);
}
