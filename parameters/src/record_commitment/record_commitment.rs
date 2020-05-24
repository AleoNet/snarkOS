pub struct RecordCommitmentParameters;

impl RecordCommitmentParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./record_commitment.params");
        buffer.to_vec()
    }
}
