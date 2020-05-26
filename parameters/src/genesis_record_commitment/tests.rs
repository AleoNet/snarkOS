use crate::genesis_record_commitment::GenesisRecordCommitment;
use snarkos_models::parameters::Parameter;

#[test]
fn test_genesis_record_commitment() {
    let parameters = GenesisRecordCommitment::load_bytes();
    assert_eq!(GenesisRecordCommitment::SIZE, parameters.len() as u64);
}
