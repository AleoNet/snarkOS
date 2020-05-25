use snarkos_models::parameters::Parameter;

pub struct InnerSNARKVKParameters;

impl Parameter for InnerSNARKVKParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 2426;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./inner_snark_vk.params");
        buffer.to_vec()
    }
}
