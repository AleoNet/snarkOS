pub struct LedgerMerkleTreeParameters;

impl LedgerMerkleTreeParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./ledger_merkle_tree.params");
        buffer.to_vec()
    }
}
