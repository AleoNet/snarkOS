use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct InnerSNARKVKParameters;

impl Parameters for InnerSNARKVKParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 2426;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./inner_snark_vk.params");
        Ok(buffer.to_vec())
    }
}
