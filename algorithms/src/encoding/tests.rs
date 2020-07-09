use crate::encoding::Elligator2;
use snarkos_curves::edwards_bls12::*;

use snarkos_utilities::rand::UniformRand;

use rand::thread_rng;

#[test]
fn test_encoding() {
    let rng = &mut thread_rng();
    let fr_element: Fr = Fr::rand(rng);

    let encoded_element = Elligator2::<EdwardsParameters, EdwardsProjective>::encode(&fr_element).unwrap();

    let recovered_fr_element = Elligator2::<EdwardsParameters, EdwardsProjective>::decode(&encoded_element).unwrap();

    assert_eq!(fr_element, recovered_fr_element);
}
