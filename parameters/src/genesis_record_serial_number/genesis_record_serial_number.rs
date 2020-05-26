use snarkos_models::parameters::Parameter;

pub struct GenesisRecordSerialNumber;

impl Parameter for GenesisRecordSerialNumber {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 64;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("genesis_record_serial_number");
        buffer.to_vec()
    }
}
