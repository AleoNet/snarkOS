use crate::account_commitment::AccountCommitmentParameters;

#[test]
fn test_account_commitment_parameters() {
    let parameters = AccountCommitmentParameters::load_bytes();
    assert_eq!(417868, parameters.len());
}
