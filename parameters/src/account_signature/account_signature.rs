use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct AccountSignatureParameters;

impl Parameters for AccountSignatureParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 96;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./account_signature.params");
        Ok(buffer.to_vec())
    }
}
