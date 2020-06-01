pub trait Genesis {
    const CHECKSUM: &'static str;
    const SIZE: u64;

    fn load_bytes() -> Vec<u8>;
}
