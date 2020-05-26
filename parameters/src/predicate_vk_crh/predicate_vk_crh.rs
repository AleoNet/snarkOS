use snarkos_models::parameters::Parameter;

pub struct PredicateVKCRHParameters;

impl Parameter for PredicateVKCRHParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 2188956;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./predicate_vk_crh.params");
        buffer.to_vec()
    }
}
