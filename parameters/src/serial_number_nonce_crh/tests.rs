use crate::serial_number_nonce_crh::SerialNumberNonceCRHParameters;
use snarkos_models::parameters::Parameters;

#[test]
fn test_serial_number_nonce_crh_parameters() {
    let parameters = SerialNumberNonceCRHParameters::load_bytes().expect("failed to load parameters");
    assert_eq!(SerialNumberNonceCRHParameters::SIZE, parameters.len() as u64);
}
