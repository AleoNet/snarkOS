pub struct PredicateSNARKPKParameters;

impl PredicateSNARKPKParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./predicate_snark_pk.params");
        buffer.to_vec()
    }
}
