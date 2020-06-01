use crate::predicate_snark_pk::PredicateSNARKPKParameters;
use snarkos_models::parameters::Parameters;

#[test]
fn test_predicate_snark_pk_parameters() {
    let parameters = PredicateSNARKPKParameters::load_bytes().expect("failed to load parameters");
    assert_eq!(PredicateSNARKPKParameters::SIZE, parameters.len() as u64);
}
