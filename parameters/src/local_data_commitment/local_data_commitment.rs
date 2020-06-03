use snarkos_algorithms::crh::sha256::sha256;
use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

use hex;

pub struct LocalDataCommitmentParameters;

impl Parameters for LocalDataCommitmentParameters {
    const CHECKSUM: &'static str = include_str!("./local_data_commitment.checksum");
    const SIZE: u64 = 2317612;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./local_data_commitment.params");
        let checksum = hex::encode(sha256(buffer));
        match Self::CHECKSUM == checksum {
            true => Ok(buffer.to_vec()),
            false => Err(ParametersError::ChecksumMismatch(Self::CHECKSUM.into(), checksum)),
        }
    }
}
