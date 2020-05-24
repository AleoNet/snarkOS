use crate::outer_snark_vk::OuterSNARKVKParameters;

#[test]
fn test_outer_snark_vk_parameters() {
    let parameters = OuterSNARKVKParameters::load_bytes();
    assert_eq!(2924, parameters.len());
}
