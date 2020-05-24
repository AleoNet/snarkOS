use crate::value_commitment::ValueCommitmentParameters;

#[test]
fn test_value_commitment_parameters() {
    let parameters = ValueCommitmentParameters::load_bytes();
    assert_eq!(403244, parameters.len());
}
