use crate::local_data_commitment::LocalDataCommitmentParameters;
use snarkos_models::parameters::Parameters;

#[test]
fn test_local_data_commitment_parameters() {
    let parameters = LocalDataCommitmentParameters::load_bytes().expect("failed to load parameters");
    assert_eq!(LocalDataCommitmentParameters::SIZE, parameters.len() as u64);
}
