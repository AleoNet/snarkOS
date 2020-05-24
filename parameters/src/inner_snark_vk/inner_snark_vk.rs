pub struct InnerSNARKVKParameters;

impl InnerSNARKVKParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./inner_snark_vk.params");
        buffer.to_vec()
    }
}
