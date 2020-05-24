pub struct PredicateVKCRHParameters;

impl PredicateVKCRHParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./predicate_vk_crh.params");
        buffer.to_vec()
    }
}
