use snarkos_models::parameters::Parameter;

pub struct LocalDataCommitmentParameters;

impl Parameter for LocalDataCommitmentParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 2317612;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./local_data_commitment.params");
        buffer.to_vec()
    }
}
