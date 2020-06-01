use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct AccountCommitmentParameters;

impl Parameters for AccountCommitmentParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 417868;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./account_commitment.params");
        Ok(buffer.to_vec())
    }
}
