use snarkos_algorithms::crh::sha256::sha256;
use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

use hex;

pub struct RecordCommitmentParameters;

impl Parameters for RecordCommitmentParameters {
    const CHECKSUM: &'static str = include_str!("./record_commitment.checksum");
    const SIZE: u64 = 489676;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./record_commitment.params");
        let checksum = hex::encode(sha256(buffer));
        match Self::CHECKSUM == checksum {
            true => Ok(buffer.to_vec()),
            false => Err(ParametersError::ChecksumMismatch(Self::CHECKSUM.into(), checksum)),
        }
    }
}
