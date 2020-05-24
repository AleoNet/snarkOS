use crate::serial_number_nonce_crh::SerialNumberNonceCRHParameters;

#[test]
fn test_serial_number_nonce_crh_parameters() {
    let parameters = SerialNumberNonceCRHParameters::load_bytes();
    assert_eq!(295972, parameters.len());
}
