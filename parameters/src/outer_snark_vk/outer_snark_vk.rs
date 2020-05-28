use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct OuterSNARKVKParameters;

impl Parameters for OuterSNARKVKParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 2924;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./outer_snark_vk.params");
        Ok(buffer.to_vec())
    }
}
