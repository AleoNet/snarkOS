use snarkos_errors::parameters::ParametersError;

pub trait Parameters {
    const CHECKSUM: &'static str;
    const SIZE: u64;

    fn load_bytes() -> Result<Vec<u8>, ParametersError>;
}
