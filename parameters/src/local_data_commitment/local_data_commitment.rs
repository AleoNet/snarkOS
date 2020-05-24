pub struct LocalDataCommitmentParameters;

impl LocalDataCommitmentParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./local_data_commitment.params");
        buffer.to_vec()
    }
}
