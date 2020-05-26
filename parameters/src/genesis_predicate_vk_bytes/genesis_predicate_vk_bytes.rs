use snarkos_models::parameters::Parameter;

pub struct GenesisPredicateVKBytes;

impl Parameter for GenesisPredicateVKBytes {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 48;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("genesis_predicate_vk_bytes");
        buffer.to_vec()
    }
}
