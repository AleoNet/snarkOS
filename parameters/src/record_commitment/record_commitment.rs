use snarkos_models::parameters::Parameter;

pub struct RecordCommitmentParameters;

impl Parameter for RecordCommitmentParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 489676;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./record_commitment.params");
        buffer.to_vec()
    }
}
