use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct PredicateVKCRHParameters;

impl Parameters for PredicateVKCRHParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 2188956;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./predicate_vk_crh.params");
        Ok(buffer.to_vec())
    }
}
