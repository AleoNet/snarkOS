use crate::account_commitment::AccountCommitmentParameters;
use snarkos_models::parameters::Parameters;

#[test]
fn test_account_commitment_parameters() {
    let parameters = AccountCommitmentParameters::load_bytes().expect("failed to load parameters");
    assert_eq!(AccountCommitmentParameters::SIZE, parameters.len() as u64);
}
