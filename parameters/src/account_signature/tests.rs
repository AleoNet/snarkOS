use crate::account_signature::AccountSignatureParameters;

#[test]
fn test_account_signature_parameters() {
    let parameters = AccountSignatureParameters::load_bytes();
    assert_eq!(96, parameters.len());
}
