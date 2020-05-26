use snarkos_models::parameters::Parameter;

pub struct OuterSNARKPKParameters;

impl Parameter for OuterSNARKPKParameters {
    const SIZE: u64 = 0;
    const CHECKSUM: &'static str = "";

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./outer_snark_pk.params");
        buffer.to_vec()
    }
}
