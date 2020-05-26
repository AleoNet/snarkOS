use snarkos_models::parameters::Parameter;

pub struct LedgerMerkleTreeParameters;

impl Parameter for LedgerMerkleTreeParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 65556;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./ledger_merkle_tree.params");
        buffer.to_vec()
    }
}
