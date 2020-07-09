use crate::encoding::Elligator2;
use snarkos_curves::{
    edwards_bls12::*,
    templates::twisted_edwards_extended::tests::{edwards_test, montgomery_conversion_test},
};

use snarkos_utilities::{rand::UniformRand, to_bytes, ToBytes};

use rand::thread_rng;

#[test]
fn test_encoding() {
    let rng = &mut thread_rng();
    let fr_element: Fr = Fr::rand(rng);

    let y = Elligator2::encode::<EdwardsParameters, EdwardsProjective>(fr_element).unwrap();
}
