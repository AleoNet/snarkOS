pub struct AccountSignatureParameters;

impl AccountSignatureParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./account_signature.params");
        buffer.to_vec()
    }
}
