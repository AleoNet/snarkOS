pub struct PredicateSNARKVKParameters;

impl PredicateSNARKVKParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./predicate_snark_vk.params");
        buffer.to_vec()
    }
}
