use crate::outer_snark_pk::OuterSNARKPKParameters;
use snarkos_models::parameters::Parameter;

#[test]
fn test_outer_snark_pk_parameters() {
    let parameters = OuterSNARKPKParameters::load_bytes();
    assert_eq!(OuterSNARKPKParameters::SIZE, parameters.len() as u64);
}
