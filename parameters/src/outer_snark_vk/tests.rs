use crate::outer_snark_vk::OuterSNARKVKParameters;
use snarkos_models::parameters::Parameters;

#[test]
fn test_outer_snark_vk_parameters() {
    let parameters = OuterSNARKVKParameters::load_bytes().expect("failed to load parameters");
    assert_eq!(OuterSNARKVKParameters::SIZE, parameters.len() as u64);
}
