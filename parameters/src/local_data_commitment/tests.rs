use crate::local_data_commitment::LocalDataCommitmentParameters;
use snarkos_models::parameters::Parameter;

#[test]
fn test_local_data_commitment_parameters() {
    let parameters = LocalDataCommitmentParameters::load_bytes();
    assert_eq!(LocalDataCommitmentParameters::SIZE, parameters.len() as u64);
}
