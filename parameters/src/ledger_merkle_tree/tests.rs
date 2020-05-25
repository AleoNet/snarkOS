use crate::ledger_merkle_tree::LedgerMerkleTreeParameters;

#[test]
fn test_ledger_merkle_tree_parameters() {
    let parameters = LedgerMerkleTreeParameters::load_bytes();
    assert_eq!(65556, parameters.len());
}
