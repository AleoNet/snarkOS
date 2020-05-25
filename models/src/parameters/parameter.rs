pub trait Parameter {
    const SIZE: u64;
    const CHECKSUM: &'static str;

    fn load_bytes() -> Vec<u8>;
}
