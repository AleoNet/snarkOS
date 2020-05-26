use crate::value_commitment::ValueCommitmentParameters;
use snarkos_models::parameters::Parameter;

#[test]
fn test_value_commitment_parameters() {
    let parameters = ValueCommitmentParameters::load_bytes();
    assert_eq!(ValueCommitmentParameters::SIZE, parameters.len() as u64);
}
