use crate::inner_snark_pk::InnerSNARKPKParameters;

#[test]
fn test_inner_snark_pk_parameters() {
    let parameters = InnerSNARKPKParameters::load_bytes();
    assert_eq!(0, parameters.len());
}
