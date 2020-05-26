use snarkos_models::parameters::Parameter;

pub struct GenesisRecordCommitment;

impl Parameter for GenesisRecordCommitment {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 32;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("record_commitment.genesis");
        buffer.to_vec()
    }
}
