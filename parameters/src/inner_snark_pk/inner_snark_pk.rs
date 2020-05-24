pub struct InnerSNARKPKParameters;

impl InnerSNARKPKParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./inner_snark_pk.params");
        buffer.to_vec()
    }
}
