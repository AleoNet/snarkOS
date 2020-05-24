pub struct SerialNumberNonceCRHParameters;

impl SerialNumberNonceCRHParameters {
    pub fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./serial_number_nonce_crh.params");
        buffer.to_vec()
    }
}
