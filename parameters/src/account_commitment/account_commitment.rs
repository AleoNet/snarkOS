pub struct AccountCommitmentParameters;

impl AccountCommitmentParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./account_commitment.params");
        buffer.to_vec()
    }
}
