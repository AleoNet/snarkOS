use crate::inner_snark_pk::InnerSNARKPKParameters;
use snarkos_models::parameters::Parameter;

#[test]
fn test_inner_snark_pk_parameters() {
    let parameters = InnerSNARKPKParameters::load_bytes();
    assert_eq!(InnerSNARKPKParameters::SIZE, parameters.len() as u64);
}
