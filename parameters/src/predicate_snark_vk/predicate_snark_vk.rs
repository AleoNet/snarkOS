use snarkos_models::parameters::Parameter;

pub struct PredicateSNARKVKParameters;

impl Parameter for PredicateSNARKVKParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 1359;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./predicate_snark_vk.params");
        buffer.to_vec()
    }
}
