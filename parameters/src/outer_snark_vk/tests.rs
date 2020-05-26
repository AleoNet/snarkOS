use crate::outer_snark_vk::OuterSNARKVKParameters;
use snarkos_models::parameters::Parameter;

#[test]
fn test_outer_snark_vk_parameters() {
    let parameters = OuterSNARKVKParameters::load_bytes();
    assert_eq!(OuterSNARKVKParameters::SIZE, parameters.len() as u64);
}
