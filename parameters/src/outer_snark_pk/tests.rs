use crate::outer_snark_pk::OuterSNARKPKParameters;

#[test]
fn test_outer_snark_pk_parameters() {
    let parameters = OuterSNARKPKParameters::load_bytes();
    assert_eq!(0, parameters.len());
}
