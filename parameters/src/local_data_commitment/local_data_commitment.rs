use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct LocalDataCommitmentParameters;

impl Parameters for LocalDataCommitmentParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 2317612;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./local_data_commitment.params");
        Ok(buffer.to_vec())
    }
}
