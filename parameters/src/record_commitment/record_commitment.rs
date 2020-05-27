use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct RecordCommitmentParameters;

impl Parameters for RecordCommitmentParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 489676;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./record_commitment.params");
        Ok(buffer.to_vec())
    }
}
