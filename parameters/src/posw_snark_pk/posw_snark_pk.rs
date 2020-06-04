use snarkos_algorithms::crh::sha256::sha256;
use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct PoswSNARKPKParameters;

impl Parameters for PoswSNARKPKParameters {
    const CHECKSUM: &'static str = include_str!("./posw_snark_pk.checksum");
    const SIZE: u64 = 26204306;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./posw_snark_pk.params");
        let checksum = hex::encode(sha256(buffer));
        match Self::CHECKSUM == checksum {
            true => Ok(buffer.to_vec()),
            false => Err(ParametersError::ChecksumMismatch(Self::CHECKSUM.into(), checksum)),
        }
    }
}
