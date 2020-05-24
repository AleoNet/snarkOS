pub struct OuterSNARKPKParameters;

impl OuterSNARKPKParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./outer_snark_pk.params");
        buffer.to_vec()
    }
}
