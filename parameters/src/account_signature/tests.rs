use crate::account_signature::AccountSignatureParameters;
use snarkos_models::parameters::Parameter;

#[test]
fn test_account_signature_parameters() {
    let parameters = AccountSignatureParameters::load_bytes();
    assert_eq!(AccountSignatureParameters::SIZE, parameters.len() as u64);
}
