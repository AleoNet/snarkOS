use snarkos_models::genesis::Genesis;

pub struct Transaction1;

impl Genesis for Transaction1 {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 1502;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("transaction_1.genesis");
        buffer.to_vec()
    }
}
