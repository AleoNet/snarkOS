use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct SerialNumberNonceCRHParameters;

impl Parameters for SerialNumberNonceCRHParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 295972;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./serial_number_nonce_crh.params");
        Ok(buffer.to_vec())
    }
}
