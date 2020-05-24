use crate::record_commitment::RecordCommitmentParameters;

#[test]
fn test_record_commitment_parameters() {
    let parameters = RecordCommitmentParameters::load_bytes();
    assert_eq!(489676, parameters.len());
}
