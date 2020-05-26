use crate::genesis_record_serial_number::GenesisRecordSerialNumber;
use snarkos_models::parameters::Parameter;

#[test]
fn test_genesis_serial_number() {
    let parameters = GenesisRecordSerialNumber::load_bytes();
    assert_eq!(GenesisRecordSerialNumber::SIZE, parameters.len() as u64);
}
