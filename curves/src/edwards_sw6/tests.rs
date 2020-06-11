use crate::{
    edwards_sw6::*,
    templates::twisted_edwards_extended::tests::{edwards_test, montgomery_conversion_test},
};
use snarkos_models::curves::{
    tests_curve::curve_tests,
    tests_field::{field_serialization_test, field_test, primefield_test},
    tests_group::group_test,
    AffineCurve,
    ProjectiveCurve,
};

use rand;

#[test]
fn test_edwards_sw6_fr() {
    let a: Fr = rand::random();
    let b: Fr = rand::random();
    field_test(a, b);
    primefield_test::<Fr>();
    field_serialization_test::<Fr>();
}

#[test]
fn test_edwards_sw6_fq() {
    let a: Fq = rand::random();
    let b: Fq = rand::random();
    field_test(a, b);
    primefield_test::<Fq>();
    field_serialization_test::<Fq>();
}

#[test]
fn test_projective_curve() {
    curve_tests::<EdwardsProjective>();
    edwards_test::<EdwardsParameters>();
}

#[test]
fn test_projective_group() {
    for _i in 0..10 {
        let a = rand::random();
        let b = rand::random();
        group_test::<EdwardsProjective>(a, b);
    }
}

#[test]
fn test_affine_group() {
    for _i in 0..10 {
        let a: EdwardsAffine = rand::random();
        let b: EdwardsAffine = rand::random();
        group_test::<EdwardsAffine>(a, b);
    }
}

#[test]
fn test_generator() {
    let generator = EdwardsAffine::prime_subgroup_generator();
    assert!(generator.is_on_curve());
    assert!(generator.is_in_correct_subgroup_assuming_on_curve());
}

#[test]
fn test_conversion() {
    let a: EdwardsAffine = rand::random();
    let b: EdwardsAffine = rand::random();
    let a_b = {
        use snarkos_models::curves::Group;
        (a + &b).double().double()
    };
    let a_b2 = (a.into_projective() + &b.into_projective()).double().double();
    assert_eq!(a_b, a_b2.into_affine());
    assert_eq!(a_b.into_projective(), a_b2);
}

#[test]
fn test_montgomery_conversion() {
    montgomery_conversion_test::<EdwardsParameters>();
}
