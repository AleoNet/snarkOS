use crate::inner_snark_pk::InnerSNARKPKParameters;
use snarkos_models::parameters::Parameters;

#[test]
fn test_inner_snark_pk_parameters() {
    let parameters = InnerSNARKPKParameters::load_bytes().expect("failed to load parameters");
    assert_eq!(InnerSNARKPKParameters::SIZE, parameters.len() as u64);
}
