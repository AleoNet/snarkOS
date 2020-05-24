pub struct OuterSNARKVKParameters;

impl OuterSNARKVKParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./outer_snark_vk.params");
        buffer.to_vec()
    }
}
