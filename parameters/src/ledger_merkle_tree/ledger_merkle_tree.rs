use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct LedgerMerkleTreeParameters;

impl Parameters for LedgerMerkleTreeParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 65556;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./ledger_merkle_tree.params");
        Ok(buffer.to_vec())
    }
}
