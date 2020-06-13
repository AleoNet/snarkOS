use super::{GroupAffine, GroupProjective};

use snarkos_utilities::{
    io::Cursor,
    rand::UniformRand,
    serialize::{CanonicalDeserialize, CanonicalSerialize},
};

use snarkos_models::curves::{
    pairing_engine::ProjectiveCurve,
    Field,
    MontgomeryModelParameters,
    One,
    TEModelParameters,
    Zero,
};

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

pub fn montgomery_conversion_test<P>()
where
    P: TEModelParameters,
{
    // A = 2 * (a + d) / (a - d)
    let a = P::BaseField::one().double() * &(P::COEFF_A + &P::COEFF_D) * &(P::COEFF_A - &P::COEFF_D).inverse().unwrap();
    // B = 4 / (a - d)
    let b = P::BaseField::one().double().double() * &(P::COEFF_A - &P::COEFF_D).inverse().unwrap();

    assert_eq!(a, P::MontgomeryModelParameters::COEFF_A);
    assert_eq!(b, P::MontgomeryModelParameters::COEFF_B);
}

pub fn edwards_test<P: TEModelParameters>() {
    let buf_size = GroupAffine::<P>::zero().serialized_size();

    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..10 {
        let a = GroupProjective::<P>::rand(&mut rng);
        let a = a.into_affine();
        {
            let mut serialized = vec![0; buf_size];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize(&mut cursor).unwrap();

            let mut cursor = Cursor::new(&serialized[..]);
            let b = GroupAffine::<P>::deserialize(&mut cursor).unwrap();
            assert_eq!(a, b);
        }

        {
            let a = GroupAffine::<P>::zero();
            let mut serialized = vec![0; buf_size];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize(&mut cursor).unwrap();
            let mut cursor = Cursor::new(&serialized[..]);
            let b = GroupAffine::<P>::deserialize(&mut cursor).unwrap();
            assert_eq!(a, b);
        }

        {
            let a = GroupAffine::<P>::zero();
            let mut serialized = vec![0; buf_size - 1];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize(&mut cursor).unwrap_err();
        }

        {
            let serialized = vec![0; buf_size - 1];
            let mut cursor = Cursor::new(&serialized[..]);
            GroupAffine::<P>::deserialize(&mut cursor).unwrap_err();
        }

        {
            let mut serialized = vec![0; a.uncompressed_size()];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize_uncompressed(&mut cursor).unwrap();

            let mut cursor = Cursor::new(&serialized[..]);
            let b = GroupAffine::<P>::deserialize_uncompressed(&mut cursor).unwrap();
            assert_eq!(a, b);
        }

        {
            let a = GroupAffine::<P>::zero();
            let mut serialized = vec![0; a.uncompressed_size()];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize_uncompressed(&mut cursor).unwrap();
            let mut cursor = Cursor::new(&serialized[..]);
            let b = GroupAffine::<P>::deserialize_uncompressed(&mut cursor).unwrap();
            assert_eq!(a, b);
        }
    }
}
