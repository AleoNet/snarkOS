use snarkos_algorithms::crh::sha256::sha256;
use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

use hex;

pub struct PredicateVKCRHParameters;

impl Parameters for PredicateVKCRHParameters {
    const CHECKSUM: &'static str = include_str!("./predicate_vk_crh.checksum");
    const SIZE: u64 = 2188956;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./predicate_vk_crh.params");
        let checksum = hex::encode(sha256(buffer));
        match Self::CHECKSUM == checksum {
            true => Ok(buffer.to_vec()),
            false => Err(ParametersError::ChecksumMismatch(Self::CHECKSUM.into(), checksum)),
        }
    }
}
