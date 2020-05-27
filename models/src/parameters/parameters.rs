use snarkos_errors::parameters::ParametersError;

pub trait Parameters {
    const SIZE: u64;
    const CHECKSUM: &'static str;

    fn load_bytes() -> Result<Vec<u8>, ParametersError>;
}
