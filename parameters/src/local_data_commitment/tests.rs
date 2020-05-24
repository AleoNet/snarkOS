use crate::local_data_commitment::LocalDataCommitmentParameters;

#[test]
fn test_local_data_commitment_parameters() {
    let parameters = LocalDataCommitmentParameters::load_bytes();
    assert_eq!(2317612, parameters.len());
}
