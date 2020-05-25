use snarkos_models::parameters::Parameter;

pub struct InnerSNARKPKParameters;

impl Parameter for InnerSNARKPKParameters {
    const SIZE: u64 = 0;
    const CHECKSUM: &'static str = "";

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./inner_snark_pk.params");
        buffer.to_vec()
    }
}
