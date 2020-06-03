use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct PredicateSNARKPKParameters;

impl Parameters for PredicateSNARKPKParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 8806582;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./predicate_snark_pk.params");
        Ok(buffer.to_vec())
    }
}
