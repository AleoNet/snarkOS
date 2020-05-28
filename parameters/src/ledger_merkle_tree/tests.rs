use crate::ledger_merkle_tree::LedgerMerkleTreeParameters;
use snarkos_models::parameters::Parameters;

#[test]
fn test_ledger_merkle_tree_parameters() {
    let parameters = LedgerMerkleTreeParameters::load_bytes().expect("failed to load parameters");
    assert_eq!(LedgerMerkleTreeParameters::SIZE, parameters.len() as u64);
}
