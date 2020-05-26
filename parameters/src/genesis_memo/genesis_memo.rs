use snarkos_models::parameters::Parameter;

pub struct GenesisMemo;

impl Parameter for GenesisMemo {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 32;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("memo.genesis");
        buffer.to_vec()
    }
}
