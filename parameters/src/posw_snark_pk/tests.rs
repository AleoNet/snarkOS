use crate::posw_snark_pk::PoswSNARKPKParameters;
use snarkos_models::parameters::Parameters;

#[test]
fn test_posw_snark_pk_parameters() {
    let parameters = PoswSNARKPKParameters::load_bytes().expect("failed to load parameters");
    assert_eq!(PoswSNARKPKParameters::SIZE, parameters.len() as u64);
}
