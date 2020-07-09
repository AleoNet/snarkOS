use crate::encoding::Elligator2;
use snarkos_curves::edwards_bls12::*;

use snarkos_utilities::rand::UniformRand;

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

pub(crate) const ITERATIONS: usize = 10000;

#[test]
fn test_elligator2_encoding() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..ITERATIONS {
        let fr_element: Fr = Fr::rand(rng);

        let encoded_element = Elligator2::<EdwardsParameters, EdwardsProjective>::encode(&fr_element).unwrap();

        let recovered_fr_element =
            Elligator2::<EdwardsParameters, EdwardsProjective>::decode(&encoded_element).unwrap();

        assert_eq!(fr_element, recovered_fr_element);
    }
}
