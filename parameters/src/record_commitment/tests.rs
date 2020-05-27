use crate::record_commitment::RecordCommitmentParameters;
use snarkos_models::parameters::Parameter;

#[test]
fn test_record_commitment_parameters() {
    let parameters = RecordCommitmentParameters::load_bytes();
    assert_eq!(RecordCommitmentParameters::SIZE, parameters.len() as u64);
}
