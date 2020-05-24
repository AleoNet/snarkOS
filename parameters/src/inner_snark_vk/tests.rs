use crate::inner_snark_vk::InnerSNARKVKParameters;

#[test]
fn test_inner_snark_vk_parameters() {
    let parameters = InnerSNARKVKParameters::load_bytes();
    assert_eq!(2426, parameters.len());
}
