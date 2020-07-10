use crate::encoding::Elligator2;
use snarkos_curves::edwards_bls12::*;

use snarkos_utilities::rand::UniformRand;

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

pub(crate) const ITERATIONS: usize = 10000;

#[test]
fn test_elligator2_encode_decode() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..ITERATIONS {
        let original: Fq = Fq::rand(rng);

        //        let encoded = Elligator2::<EdwardsParameters, EdwardsProjective>::encode(&original).unwrap();
        //        let decoded = Elligator2::<EdwardsParameters, EdwardsProjective>::decode(&encoded).unwrap();
        //
        //        match original == decoded {
        //            true => {
        //                println!("{} == {}", original, decoded);
        //                assert_eq!(original, decoded)
        //            },
        //            false => {
        //                println!("{} == {}", original, -decoded);
        //                assert_eq!(original, -decoded)
        //            },
        //        }

        let (encoded, fq_high) = Elligator2::<EdwardsParameters, EdwardsProjective>::encode(&original).unwrap();
        let decoded = Elligator2::<EdwardsParameters, EdwardsProjective>::decode(&encoded, fq_high).unwrap();

        println!("{} == {}", original, decoded);
        assert_eq!(original, decoded)
    }
}
