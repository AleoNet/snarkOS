use snarkos_models::parameters::Parameter;

pub struct PredicateSNARKPKParameters;

impl Parameter for PredicateSNARKPKParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 8806582;

    fn load_bytes() -> Vec<u8> {
        let buffer = include_bytes!("./predicate_snark_pk.params");
        buffer.to_vec()
    }
}
