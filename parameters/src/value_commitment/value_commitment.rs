use snarkos_models::parameters::Parameter;

pub struct ValueCommitmentParameters;

impl Parameter for ValueCommitmentParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 403244;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./value_commitment.params");
        buffer.to_vec()
    }
}
