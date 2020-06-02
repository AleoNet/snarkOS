use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct PredicateSNARKVKParameters;

impl Parameters for PredicateSNARKVKParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 1068;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./predicate_snark_vk.params");
        Ok(buffer.to_vec())
    }
}
