use snarkos_models::parameters::Parameter;

pub struct OuterSNARKVKParameters;

impl Parameter for OuterSNARKVKParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 2924;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./outer_snark_vk.params");
        buffer.to_vec()
    }
}
