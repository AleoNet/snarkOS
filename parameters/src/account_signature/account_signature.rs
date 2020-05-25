use snarkos_models::parameters::Parameter;

pub struct AccountSignatureParameters;

impl Parameter for AccountSignatureParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 96;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./account_signature.params");
        buffer.to_vec()
    }
}
