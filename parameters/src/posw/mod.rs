use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

pub struct PoswProvingParameters;

impl Parameters for PoswProvingParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 26204306;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./posw_pk.params");
        Ok(buffer.to_vec())
    }
}

pub struct PoswVerificationParameters;

impl Parameters for PoswVerificationParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 1165;

    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        let buffer = include_bytes!("./posw_vk.params");
        Ok(buffer.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proving_parameters() {
        let parameters = PoswProvingParameters::load_bytes().expect("failed to load parameters");
        assert_eq!(PoswProvingParameters::SIZE, parameters.len() as u64);
    }

    #[test]
    fn test_verification_parameters() {
        let parameters = PoswVerificationParameters::load_bytes().expect("failed to load parameters");
        assert_eq!(PoswVerificationParameters::SIZE, parameters.len() as u64);
    }
}
