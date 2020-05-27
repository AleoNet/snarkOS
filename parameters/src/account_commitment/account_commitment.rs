use snarkos_models::parameters::Parameter;

pub struct AccountCommitmentParameters;

impl Parameter for AccountCommitmentParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 417868;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./account_commitment.params");
        buffer.to_vec()
    }
}
