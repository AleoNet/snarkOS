use crate::value_commitment::ValueCommitmentParameters;
use snarkos_models::parameters::Parameters;

#[test]
fn test_value_commitment_parameters() {
    let parameters = ValueCommitmentParameters::load_bytes().expect("failed to load parameters");
    assert_eq!(ValueCommitmentParameters::SIZE, parameters.len() as u64);
}
