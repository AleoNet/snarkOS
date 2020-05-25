use snarkos_models::parameters::Parameter;

pub struct SerialNumberNonceCRHParameters;

impl Parameter for SerialNumberNonceCRHParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 295972;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./serial_number_nonce_crh.params");
        buffer.to_vec()
    }
}
