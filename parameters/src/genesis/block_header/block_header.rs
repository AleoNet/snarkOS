use snarkos_models::genesis::Genesis;

pub struct GenesisBlockHeader;

impl Genesis for GenesisBlockHeader {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 1088;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("block_header.genesis");
        buffer.to_vec()
    }
}
