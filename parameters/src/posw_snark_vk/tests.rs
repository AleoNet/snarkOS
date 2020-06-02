use crate::posw_snark_vk::PoswSNARKVKParameters;
use snarkos_models::parameters::Parameters;

#[test]
fn test_posw_snark_vk_parameters() {
    let parameters = PoswSNARKVKParameters::load_bytes().expect("failed to load parameters");
    assert_eq!(PoswSNARKVKParameters::SIZE, parameters.len() as u64);
}
