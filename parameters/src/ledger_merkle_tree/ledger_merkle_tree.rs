use snarkos_algorithms::crh::sha256::sha256;
use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

use hex;

pub struct LedgerMerkleTreeParameters;

impl Parameters for LedgerMerkleTreeParameters {
    const CHECKSUM: &'static str = include_str!("./ledger_merkle_tree.checksum");
    const SIZE: u64 = 65556;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./ledger_merkle_tree.params");
        let checksum = hex::encode(sha256(buffer));
        match Self::CHECKSUM == checksum {
            true => Ok(buffer.to_vec()),
            false => Err(ParametersError::ChecksumMismatch(Self::CHECKSUM.into(), checksum)),
        }
    }
}
