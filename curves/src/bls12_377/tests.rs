#![allow(unused_imports)]
use crate::{
    bls12_377::{
        g1::Bls12_377G1Parameters,
        g2::Bls12_377G2Parameters,
        Bls12_377,
        Fq,
        Fq12,
        Fq2,
        Fq2Parameters,
        Fq6,
        Fq6Parameters,
        FqParameters,
        Fr,
        G1Affine,
        G1Projective,
        G2Affine,
        G2Projective,
    },
    templates::{short_weierstrass::tests::sw_tests, twisted_edwards_extended::tests::edwards_test},
};
use snarkos_models::curves::{
    fp6_3over2::Fp6Parameters,
    tests_curve::curve_tests,
    tests_field::{field_serialization_test, field_test, frobenius_test, primefield_test, sqrt_field_test},
    tests_group::group_test,
    AffineCurve,
    Field,
    Fp2Parameters,
    FpParameters,
    LegendreSymbol::*,
    One,
    PairingEngine,
    PrimeField,
    ProjectiveCurve,
    SWModelParameters,
    SquareRootField,
    Zero,
};
use snarkos_utilities::{
    biginteger::{BigInteger, BigInteger384},
    rand::UniformRand,
};

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use std::{
    cmp::Ordering,
    ops::{AddAssign, MulAssign, SubAssign},
};

pub(crate) const ITERATIONS: usize = 5;

#[test]
fn test_bls12_377_fr() {
    for _ in 0..ITERATIONS {
        let a: Fr = rand::random();
        let b: Fr = rand::random();
        field_test(a, b);
        primefield_test::<Fr>();
        sqrt_field_test(b);
        field_serialization_test::<Fr>();
    }
}

#[test]
fn test_bls12_377_fq() {
    for _ in 0..ITERATIONS {
        let a: Fq = rand::random();
        let b: Fq = rand::random();
        field_test(a, b);
        primefield_test::<Fq>();
        sqrt_field_test(a);
        field_serialization_test::<Fq>();
    }
}

#[test]
fn test_bls12_377_fq2() {
    for _ in 0..ITERATIONS {
        let a: Fq2 = rand::random();
        let b: Fq2 = rand::random();
        field_test(a, b);
        sqrt_field_test(a);
    }
    frobenius_test::<Fq2, _>(Fq::characteristic(), 13);
    field_serialization_test::<Fq2>();
}

#[test]
fn test_bls12_377_fq6() {
    for _ in 0..ITERATIONS {
        let g: Fq6 = rand::random();
        let h: Fq6 = rand::random();
        field_test(g, h);
    }
    frobenius_test::<Fq6, _>(Fq::characteristic(), 13);
    field_serialization_test::<Fq6>();
}

#[test]
fn test_bls12_377_fq12() {
    for _ in 0..ITERATIONS {
        let g: Fq12 = rand::random();
        let h: Fq12 = rand::random();
        field_test(g, h);
    }
    frobenius_test::<Fq12, _>(Fq::characteristic(), 13);
    field_serialization_test::<Fq12>();
}

#[test]
fn test_fq_repr_from() {
    assert_eq!(BigInteger384::from(100), BigInteger384([100, 0, 0, 0, 0, 0]));
}

#[test]
fn test_fq_repr_is_odd() {
    assert!(!BigInteger384::from(0).is_odd());
    assert!(BigInteger384::from(0).is_even());
    assert!(BigInteger384::from(1).is_odd());
    assert!(!BigInteger384::from(1).is_even());
    assert!(!BigInteger384::from(324834872).is_odd());
    assert!(BigInteger384::from(324834872).is_even());
    assert!(BigInteger384::from(324834873).is_odd());
    assert!(!BigInteger384::from(324834873).is_even());
}

#[test]
fn test_fq_repr_is_zero() {
    assert!(BigInteger384::from(0).is_zero());
    assert!(!BigInteger384::from(1).is_zero());
    assert!(!BigInteger384([0, 0, 0, 0, 1, 0]).is_zero());
}

#[test]
fn test_fq_repr_num_bits() {
    let mut a = BigInteger384::from(0);
    assert_eq!(0, a.num_bits());
    a = BigInteger384::from(1);
    for i in 1..385 {
        assert_eq!(i, a.num_bits());
        a.mul2();
    }
    assert_eq!(0, a.num_bits());
}

#[test]
fn test_fq_add_assign() {
    // Test associativity

    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        // Generate a, b, c and ensure (a + b) + c == a + (b + c).
        let a = Fq::rand(&mut rng);
        let b = Fq::rand(&mut rng);
        let c = Fq::rand(&mut rng);

        let mut tmp1 = a;
        tmp1.add_assign(&b);
        tmp1.add_assign(&c);

        let mut tmp2 = b;
        tmp2.add_assign(&c);
        tmp2.add_assign(&a);

        assert!(tmp1.is_valid());
        assert!(tmp2.is_valid());
        assert_eq!(tmp1, tmp2);
    }
}

#[test]
fn test_fq_sub_assign() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        // Ensure that (a - b) + (b - a) = 0.
        let a = Fq::rand(&mut rng);
        let b = Fq::rand(&mut rng);

        let mut tmp1 = a;
        tmp1.sub_assign(&b);

        let mut tmp2 = b;
        tmp2.sub_assign(&a);

        tmp1.add_assign(&tmp2);
        assert!(tmp1.is_zero());
    }
}

#[test]
fn test_fq_mul_assign() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000000 {
        // Ensure that (a * b) * c = a * (b * c)
        let a = Fq::rand(&mut rng);
        let b = Fq::rand(&mut rng);
        let c = Fq::rand(&mut rng);

        let mut tmp1 = a;
        tmp1.mul_assign(&b);
        tmp1.mul_assign(&c);

        let mut tmp2 = b;
        tmp2.mul_assign(&c);
        tmp2.mul_assign(&a);

        assert_eq!(tmp1, tmp2);
    }

    for _ in 0..1000000 {
        // Ensure that r * (a + b + c) = r*a + r*b + r*c

        let r = Fq::rand(&mut rng);
        let mut a = Fq::rand(&mut rng);
        let mut b = Fq::rand(&mut rng);
        let mut c = Fq::rand(&mut rng);

        let mut tmp1 = a;
        tmp1.add_assign(&b);
        tmp1.add_assign(&c);
        tmp1.mul_assign(&r);

        a.mul_assign(&r);
        b.mul_assign(&r);
        c.mul_assign(&r);

        a.add_assign(&b);
        a.add_assign(&c);

        assert_eq!(tmp1, a);
    }
}

#[test]
fn test_fq_squaring() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000000 {
        // Ensure that (a * a) = a^2
        let a = Fq::rand(&mut rng);

        let mut tmp = a;
        tmp.square_in_place();

        let mut tmp2 = a;
        tmp2.mul_assign(&a);

        assert_eq!(tmp, tmp2);
    }
}

#[test]
fn test_fq_inverse() {
    assert!(Fq::zero().inverse().is_none());

    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    let one = Fq::one();

    for _ in 0..1000 {
        // Ensure that a * a^-1 = 1
        let mut a = Fq::rand(&mut rng);
        let ainv = a.inverse().unwrap();
        a.mul_assign(&ainv);
        assert_eq!(a, one);
    }
}

#[test]
fn test_fq_double_in_place() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        // Ensure doubling a is equivalent to adding a to itself.
        let mut a = Fq::rand(&mut rng);
        let mut b = a;
        b.add_assign(&a);
        a.double_in_place();
        assert_eq!(a, b);
    }
}

#[test]
fn test_fq_negate() {
    {
        let a = -Fq::zero();

        assert!(a.is_zero());
    }

    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        // Ensure (a - (-a)) = 0.
        let mut a = Fq::rand(&mut rng);
        let b = -a;
        a.add_assign(&b);

        assert!(a.is_zero());
    }
}

#[test]
fn test_fq_pow() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for i in 0..1000 {
        // Exponentiate by various small numbers and ensure it consists with repeated
        // multiplication.
        let a = Fq::rand(&mut rng);
        let target = a.pow(&[i]);
        let mut c = Fq::one();
        for _ in 0..i {
            c.mul_assign(&a);
        }
        assert_eq!(c, target);
    }

    for _ in 0..1000 {
        // Exponentiating by the modulus should have no effect in a prime field.
        let a = Fq::rand(&mut rng);

        assert_eq!(a, a.pow(Fq::characteristic()));
    }
}

#[test]
fn test_fq_sqrt() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    assert_eq!(Fq::zero().sqrt().unwrap(), Fq::zero());

    for _ in 0..1000 {
        // Ensure sqrt(a^2) = a or -a
        let a = Fq::rand(&mut rng);
        let nega = -a;
        let mut b = a;
        b.square_in_place();

        let b = b.sqrt().unwrap();

        assert!(a == b || nega == b);
    }

    for _ in 0..1000 {
        // Ensure sqrt(a)^2 = a for random a
        let a = Fq::rand(&mut rng);

        if let Some(mut tmp) = a.sqrt() {
            tmp.square_in_place();

            assert_eq!(a, tmp);
        }
    }
}

#[test]
fn test_fq_num_bits() {
    assert_eq!(FqParameters::MODULUS_BITS, 377);
    assert_eq!(FqParameters::CAPACITY, 376);
}

#[test]
fn test_fq_root_of_unity() {
    assert_eq!(FqParameters::TWO_ADICITY, 46);
    assert_eq!(
        Fq::multiplicative_generator().pow([
            0x7510c00000021423,
            0x88bee82520005c2d,
            0x67cc03d44e3c7bcd,
            0x1701b28524ec688b,
            0xe9185f1443ab18ec,
            0x6b8
        ]),
        Fq::root_of_unity()
    );
    assert_eq!(Fq::root_of_unity().pow([1 << FqParameters::TWO_ADICITY]), Fq::one());
    assert!(Fq::multiplicative_generator().sqrt().is_none());
}

#[test]
fn test_fq_ordering() {
    // BigInteger384's ordering is well-tested, but we still need to make sure the
    // Fq elements aren't being compared in Montgomery form.
    for i in 0..100 {
        assert!(Fq::from_repr(BigInteger384::from(i + 1)).unwrap() > Fq::from_repr(BigInteger384::from(i)).unwrap());
    }
}

#[test]
fn test_fq_legendre() {
    assert_eq!(QuadraticResidue, Fq::one().legendre());
    assert_eq!(Zero, Fq::zero().legendre());
    assert_eq!(
        QuadraticResidue,
        Fq::from_repr(BigInteger384::from(4)).unwrap().legendre()
    );
    assert_eq!(
        QuadraticNonResidue,
        Fq::from_repr(BigInteger384::from(5)).unwrap().legendre()
    );
}

#[test]
fn test_fq2_ordering() {
    let mut a = Fq2::new(Fq::zero(), Fq::zero());
    let mut b = a.clone();

    assert!(a.cmp(&b) == Ordering::Equal);
    b.c0.add_assign(&Fq::one());
    assert!(a.cmp(&b) == Ordering::Less);
    a.c0.add_assign(&Fq::one());
    assert!(a.cmp(&b) == Ordering::Equal);
    b.c1.add_assign(&Fq::one());
    assert!(a.cmp(&b) == Ordering::Less);
    a.c0.add_assign(&Fq::one());
    assert!(a.cmp(&b) == Ordering::Less);
    a.c1.add_assign(&Fq::one());
    assert!(a.cmp(&b) == Ordering::Greater);
    b.c0.add_assign(&Fq::one());
    assert!(a.cmp(&b) == Ordering::Equal);
}

#[test]
fn test_fq2_basics() {
    assert_eq!(Fq2::new(Fq::zero(), Fq::zero(),), Fq2::zero());
    assert_eq!(Fq2::new(Fq::one(), Fq::zero(),), Fq2::one());
    assert!(Fq2::zero().is_zero());
    assert!(!Fq2::one().is_zero());
    assert!(!Fq2::new(Fq::zero(), Fq::one(),).is_zero());
}

#[test]
fn test_fq2_legendre() {
    assert_eq!(Zero, Fq2::zero().legendre());
    // i^2 = -1
    let mut m1 = -Fq2::one();
    assert_eq!(QuadraticResidue, m1.legendre());
    m1 = Fq6Parameters::mul_fp2_by_nonresidue(&m1);
    assert_eq!(QuadraticNonResidue, m1.legendre());
}

#[test]
fn test_fq2_mul_nonresidue() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    let nqr = Fq2::new(Fq::zero(), Fq::one());

    let quadratic_non_residue = Fq2::new(
        Fq2Parameters::QUADRATIC_NONRESIDUE.0,
        Fq2Parameters::QUADRATIC_NONRESIDUE.1,
    );
    for _ in 0..1000 {
        let mut a = Fq2::rand(&mut rng);
        let mut b = a;
        a = quadratic_non_residue * &a;
        b.mul_assign(&nqr);

        assert_eq!(a, b);
    }
}

#[test]
fn test_fq6_mul_by_1() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        let c1 = Fq2::rand(&mut rng);
        let mut a = Fq6::rand(&mut rng);
        let mut b = a;

        a.mul_by_1(&c1);
        b.mul_assign(&Fq6::new(Fq2::zero(), c1, Fq2::zero()));

        assert_eq!(a, b);
    }
}

#[test]
fn test_fq6_mul_by_01() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        let c0 = Fq2::rand(&mut rng);
        let c1 = Fq2::rand(&mut rng);
        let mut a = Fq6::rand(&mut rng);
        let mut b = a;

        a.mul_by_01(&c0, &c1);
        b.mul_assign(&Fq6::new(c0, c1, Fq2::zero()));

        assert_eq!(a, b);
    }
}

#[test]
fn test_fq12_mul_by_014() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        let c0 = Fq2::rand(&mut rng);
        let c1 = Fq2::rand(&mut rng);
        let c5 = Fq2::rand(&mut rng);
        let mut a = Fq12::rand(&mut rng);
        let mut b = a;

        a.mul_by_014(&c0, &c1, &c5);
        b.mul_assign(&Fq12::new(
            Fq6::new(c0, c1, Fq2::zero()),
            Fq6::new(Fq2::zero(), c5, Fq2::zero()),
        ));

        assert_eq!(a, b);
    }
}

#[test]
fn test_fq12_mul_by_034() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..1000 {
        let c0 = Fq2::rand(&mut rng);
        let c3 = Fq2::rand(&mut rng);
        let c4 = Fq2::rand(&mut rng);
        let mut a = Fq12::rand(&mut rng);
        let mut b = a;

        a.mul_by_034(&c0, &c3, &c4);
        b.mul_assign(&Fq12::new(
            Fq6::new(c0, Fq2::zero(), Fq2::zero()),
            Fq6::new(c3, c4, Fq2::zero()),
        ));

        assert_eq!(a, b);
    }
}

#[test]
fn test_g1_projective_curve() {
    curve_tests::<G1Projective>();
    sw_tests::<Bls12_377G1Parameters>();
}

#[test]
fn test_g1_projective_group() {
    let a: G1Projective = rand::random();
    let b: G1Projective = rand::random();
    group_test(a, b);
}

#[test]
fn test_g1_generator() {
    let generator = G1Affine::prime_subgroup_generator();
    assert!(generator.is_on_curve());
    assert!(generator.is_in_correct_subgroup_assuming_on_curve());
}

#[test]
fn test_g2_projective_curve() {
    curve_tests::<G2Projective>();
    sw_tests::<Bls12_377G2Parameters>();
}

#[test]
fn test_g2_projective_group() {
    let a: G2Projective = rand::random();
    let b: G2Projective = rand::random();
    group_test(a, b);
}

#[test]
fn test_g2_generator() {
    let generator = G2Affine::prime_subgroup_generator();
    assert!(generator.is_on_curve());
    assert!(generator.is_in_correct_subgroup_assuming_on_curve());
}

#[test]
fn test_bilinearity() {
    let a: G1Projective = rand::random();
    let b: G2Projective = rand::random();
    let s: Fr = rand::random();

    let sa = a * &s;
    let sb = b * &s;

    let ans1 = Bls12_377::pairing(sa, b);
    let ans2 = Bls12_377::pairing(a, sb);
    let ans3 = Bls12_377::pairing(a, b).pow(s.into_repr());

    assert_eq!(ans1, ans2);
    assert_eq!(ans2, ans3);

    assert_ne!(ans1, Fq12::one());
    assert_ne!(ans2, Fq12::one());
    assert_ne!(ans3, Fq12::one());

    assert_eq!(ans1.pow(Fr::characteristic()), Fq12::one());
    assert_eq!(ans2.pow(Fr::characteristic()), Fq12::one());
    assert_eq!(ans3.pow(Fr::characteristic()), Fq12::one());
}

#[test]
fn test_g1_generator_raw() {
    let mut x = Fq::zero();
    let mut i = 0;
    loop {
        // y^2 = x^3 + b
        let mut rhs = x;
        rhs.square_in_place();
        rhs.mul_assign(&x);
        rhs.add_assign(&Bls12_377G1Parameters::COEFF_B);

        if let Some(y) = rhs.sqrt() {
            let p = G1Affine::new(x, if y < -y { y } else { -y }, false);
            assert!(!p.is_in_correct_subgroup_assuming_on_curve());

            let g1 = p.scale_by_cofactor();
            if !g1.is_zero() {
                assert_eq!(i, 1);
                let g1 = G1Affine::from(g1);

                assert!(g1.is_in_correct_subgroup_assuming_on_curve());

                assert_eq!(g1, G1Affine::prime_subgroup_generator());
                break;
            }
        }

        i += 1;
        x.add_assign(&Fq::one());
    }
}
