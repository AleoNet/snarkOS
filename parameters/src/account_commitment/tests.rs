use crate::account_commitment::AccountCommitmentParameters;
use snarkos_models::parameters::Parameter;

#[test]
fn test_account_commitment_parameters() {
    let parameters = AccountCommitmentParameters::load_bytes();
    assert_eq!(AccountCommitmentParameters::SIZE, parameters.len() as u64);
}
