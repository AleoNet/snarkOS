use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct ValueCommitmentParameters;

impl Parameters for ValueCommitmentParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 403244;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./value_commitment.params");
        Ok(buffer.to_vec())
    }
}
