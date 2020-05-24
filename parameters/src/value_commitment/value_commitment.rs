pub struct ValueCommitmentParameters;

impl ValueCommitmentParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./value_commitment.params");
        buffer.to_vec()
    }
}
