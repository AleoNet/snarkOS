use crate::encoding::Elligator2;
use snarkos_curves::edwards_bls12::*;

use snarkos_utilities::rand::UniformRand;

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

pub(crate) const ITERATIONS: usize = 5;

#[test]
fn test_elligator2_encode_decode() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..ITERATIONS {
        let original: Fq = Fq::rand(rng);

        let (encoded, fq_high) = Elligator2::<EdwardsParameters, EdwardsProjective>::encode(&original).unwrap();
        let decoded = Elligator2::<EdwardsParameters, EdwardsProjective>::decode(&encoded, fq_high).unwrap();

        //        println!("\n{} == {}", original, decoded);

        use snarkos_models::curves::{AffineCurve, ProjectiveCurve};
        let encoded_x = encoded.x;

        println!("encoded: {:?}", encoded);

        //        let x_bytes = to_bytes![encoded_x].unwrap();

        if let Some(affine) = <EdwardsProjective as ProjectiveCurve>::Affine::from_x_coordinate(encoded_x.clone(), true)
        {
            if affine.is_in_correct_subgroup_assuming_on_curve() {
                println!("affine 1: {:?}", affine);
            }
        }

        if let Some(affine) =
            <EdwardsProjective as ProjectiveCurve>::Affine::from_x_coordinate(encoded_x.clone(), false)
        {
            if affine.is_in_correct_subgroup_assuming_on_curve() {
                println!("affine 1: {:?}", affine);
            }
        }

        //
        //        let greatest = match <EdwardsProjective as ProjectiveCurve>::Affine::from_x_coordinate(encoded_x.clone(), true) {
        //            Some(affine) => encoded == affine,
        //            None => false,
        //        };
        //
        //        let recovered = <EdwardsProjective as ProjectiveCurve>::Affine::from_x_coordinate(encoded_x, greatest).unwrap();

        assert_eq!(encoded, recovered);

        assert_eq!(original, decoded)
    }
}
