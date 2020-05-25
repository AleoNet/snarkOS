use crate::serial_number_nonce_crh::SerialNumberNonceCRHParameters;
use snarkos_models::parameters::Parameter;

#[test]
fn test_serial_number_nonce_crh_parameters() {
    let parameters = SerialNumberNonceCRHParameters::load_bytes();
    assert_eq!(SerialNumberNonceCRHParameters::SIZE, parameters.len() as u64);
}
