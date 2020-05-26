use crate::inner_snark_vk::InnerSNARKVKParameters;
use snarkos_models::parameters::Parameter;

#[test]
fn test_inner_snark_vk_parameters() {
    let parameters = InnerSNARKVKParameters::load_bytes();
    assert_eq!(InnerSNARKVKParameters::SIZE, parameters.len() as u64);
}
