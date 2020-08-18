// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{
    edwards_bls12::*,
    templates::twisted_edwards_extended::tests::{edwards_test, montgomery_conversion_test},
};
use snarkos_models::curves::{
    tests_curve::curve_tests,
    tests_field::{field_serialization_test, field_test, primefield_test},
    tests_group::group_test,
    AffineCurve,
    Field,
    LegendreSymbol,
    MontgomeryModelParameters,
    One,
    ProjectiveCurve,
    SquareRootField,
    TEModelParameters,
    Zero,
};
use snarkos_utilities::{rand::UniformRand, to_bytes, ToBytes};

use rand::thread_rng;

#[test]
fn test_edwards_bls12_fr() {
    let a: Fr = rand::random();
    let b: Fr = rand::random();
    field_test(a, b);
    primefield_test::<Fr>();
    field_serialization_test::<Fr>();
}

#[test]
fn test_edwards_bls12_fq() {
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

#[test]
fn test_edwards_to_montgomery_point() {
    let a: EdwardsAffine = rand::random();
    let (x, y) = (a.x, a.y);

    // Montgomery element (u, v)
    let (u, v) = {
        let numerator = Fq::one() + &y;
        let denominator = Fq::one() - &y;

        let u = numerator.clone() * &(denominator.inverse().unwrap());
        let v = numerator * &((denominator * &x).inverse().unwrap());
        (u, v)
    };

    // Ensure (u, v) is a valid Montgomery element
    {
        const A: Fq = <EdwardsParameters as MontgomeryModelParameters>::COEFF_A;
        const B: Fq = <EdwardsParameters as MontgomeryModelParameters>::COEFF_B;

        // Enforce B * v^2 == u^3 + A * u^2 + u
        let v2 = v.square();
        let u2 = u.square();
        let u3 = u2 * &u;
        assert_eq!(B * &v2, u3 + &(A * &u2) + &u);
    }

    // Edwards element (x, y)
    let (x_reconstructed, y_reconstructed) = {
        let x = u * &v.inverse().unwrap();

        let numerator = u - &Fq::one();
        let denominator = u + &Fq::one();
        let y = numerator * &denominator.inverse().unwrap();

        (x, y)
    };

    assert_eq!(x, x_reconstructed);
    assert_eq!(y, y_reconstructed);
}

#[ignore]
#[test]
fn print_montgomery_to_weierstrass_parameters() {
    const A: Fq = <EdwardsParameters as MontgomeryModelParameters>::COEFF_A;
    const B: Fq = <EdwardsParameters as MontgomeryModelParameters>::COEFF_B;

    let two = Fq::one() + &Fq::one();
    let three = Fq::one() + &two;
    let nine = three.clone() + &(three.clone() + &three);
    let twenty_seven = nine.clone() + &(nine.clone() + &nine);

    let a2 = A.square();
    let a3 = A * &a2;
    let b2 = B.square();
    let b3 = B * &b2;

    // Let a = (3 - A^2) / (3 * B^2).
    let numerator = three.clone() - &a2;
    let denominator = three * &b2;
    let a = numerator * &denominator.inverse().unwrap();

    // Let b = (2 * A^3 - 9 * A) / (27 * B^3).
    let numerator = (two * &a3) - &(nine * &A);
    let denominator = twenty_seven * &b3;
    let b = numerator * &denominator.inverse().unwrap();

    println!("A - {}\nB - {}", a, b);
}

#[test]
fn test_isomorphism() {
    let rng = &mut thread_rng();

    // Sample a random Fr element.
    let fr_element: Fr = Fr::rand(rng);

    println!("Starting Fr element is - {:?}", fr_element);

    // Map it to its corresponding Fq element.
    let fq_element = {
        let output = Fq::from_random_bytes(&to_bytes![fr_element].unwrap());
        assert!(output.is_some());
        output.unwrap()
    };

    println!("Starting Fq element is {:?}", fq_element);

    // Declare the parameters for the Montgomery equation: B * v^2 == u^3 + A * u^2 + u.
    const A: Fq = <EdwardsParameters as MontgomeryModelParameters>::COEFF_A;
    const B: Fq = <EdwardsParameters as MontgomeryModelParameters>::COEFF_B;

    // Compute the parameters for the alternate Montgomery form: v^2 == u^3 + A * u^2 + B * u.
    let (a, b) = {
        let a = A * &B.inverse().unwrap();
        let b = Fq::one() * &B.square().inverse().unwrap();
        (a, b)
    };

    // Compute the mapping from Fq to E(Fq) as an alternate Montgomery element (u, v).
    let (u, v) = {
        // Let r = element.
        let r = fq_element;

        // Let u = D.
        // TODO (howardwu): change to 5.
        let u = <EdwardsParameters as TEModelParameters>::COEFF_D;

        // Let ur2 = u * r^2;
        let ur2 = r.square() * &u;

        {
            // Verify r is nonzero.
            assert!(!r.is_zero());

            // Verify u is a quadratic nonresidue.
            assert!(u.legendre().is_qnr());

            // Verify 1 + ur^2 != 0.
            assert_ne!(Fq::one() + &ur2, Fq::zero());

            // Verify A^2 * ur^2 != B(1 + ur^2)^2.
            let a2 = a.square();
            assert_ne!(a2 * &ur2, (Fq::one() + &ur2).square() * &b);
        }

        // Let v = -A / (1 + ur^2).
        let v = (Fq::one() + &ur2).inverse().unwrap() * &(-a);

        // Let e = legendre(v^3 + Av^2 + Bv).
        let v2 = v.square();
        let v3 = v2 * &v;
        let av2 = a.clone() * &v2;
        let bv = b.clone() * &v;
        let e = (v3 + &(av2 + &bv)).legendre();

        // Let x = ev - ((1 - e) * A/2).
        let two = Fq::one().double();
        let x = match e {
            LegendreSymbol::Zero => -(a.clone() * &two.inverse().unwrap()),
            LegendreSymbol::QuadraticResidue => v,
            LegendreSymbol::QuadraticNonResidue => (-v) - &a,
        };

        // Let y = -e * sqrt(x^3 + Ax^2 + Bx).
        let x2 = x.square();
        let x3 = x2 * &x;
        let ax2 = a.clone() * &x2;
        let bx = b.clone() * &x;
        let value = (x3 + &(ax2 + &bx)).sqrt().unwrap();
        let y = match e {
            LegendreSymbol::Zero => Fq::zero(),
            LegendreSymbol::QuadraticResidue => -value,
            LegendreSymbol::QuadraticNonResidue => value,
        };

        (x, y)
    };

    // Ensure (u, v) is a valid alternate Montgomery element.
    {
        // Enforce v^2 == u^3 + A * u^2 + B * u
        let v2 = v.square();
        let u2 = u.square();
        let u3 = u2 * &u;
        assert_eq!(v2, u3 + &(a * &u2) + &(b * &u));
    }

    // Convert the alternate Montgomery element (u, v) to Montgomery element (s, t).
    let (s, t) = {
        let s = u * &B;
        let t = v * &B;

        // Ensure (s, t) is a valid Montgomery element
        {
            // Enforce B * t^2 == s^3 + A * s^2 + s
            let t2 = t.square();
            let s2 = s.square();
            let s3 = s2 * &s;
            assert_eq!(B * &t2, s3 + &(A * &s2) + &s);
        }

        (s, t)
    };

    // Convert the Montgomery element (s, t) to the twisted Edwards element (x, y).
    let (x, y) = {
        let x = s * &t.inverse().unwrap();

        let numerator = s - &Fq::one();
        let denominator = s + &Fq::one();
        let y = numerator * &denominator.inverse().unwrap();

        (x, y)
    };

    let group = EdwardsAffine::new(x, y);

    println!("{:?}", group);

    // Convert the twisted Edwards element (x, y) to the alternate Montgomery element (u, v)
    let (u_reconstructed, v_reconstructed) = {
        let numerator = Fq::one() + &y;
        let denominator = Fq::one() - &y;

        let u = numerator.clone() * &(denominator.inverse().unwrap());
        let v = numerator * &((denominator * &x).inverse().unwrap());

        // Ensure (u, v) is a valid Montgomery element
        {
            // Enforce B * v^2 == u^3 + A * u^2 + u
            let v2 = v.square();
            let u2 = u.square();
            let u3 = u2 * &u;
            assert_eq!(B * &v2, u3 + &(A * &u2) + &u);
        }

        let u = u * &B.inverse().unwrap();
        let v = v * &B.inverse().unwrap();

        // Ensure (u, v) is a valid alternate Montgomery element.
        {
            // Enforce v^2 == u^3 + A * u^2 + B * u
            let v2 = v.square();
            let u2 = u.square();
            let u3 = u2 * &u;
            assert_eq!(v2, u3 + &(a * &u2) + &(b * &u));
        }

        (u, v)
    };

    assert_eq!(u, u_reconstructed);
    assert_eq!(v, v_reconstructed);

    let fq_element_reconstructed = {
        let x = u_reconstructed;

        // TODO (howardwu): change to 5.
        // Let u = D.
        let u = <EdwardsParameters as TEModelParameters>::COEFF_D;

        {
            // Verify u is a quadratic nonresidue.
            assert!(u.legendre().is_qnr());

            // Verify that x != -A.
            assert_ne!(x, -a);

            // Verify that if y is 0, then x is 0.
            if y.is_zero() {
                assert!(x.is_zero());
            }

            // Verify -ux(x + A) is a residue.
            assert_eq!((-(u * &x) * &(x + &a)).legendre(), LegendreSymbol::QuadraticResidue);
        }

        println!("\ngroup legendre - {:?}", y.legendre());

        // Let value1 = sqrt(-x / ((x + A) * u)).
        let numerator = -x;
        let denominator = (x + &a) * &u;
        let value1 = (numerator * &denominator.inverse().unwrap()).sqrt();

        // Let value2 = sqrt(-(x + A) / ux)).
        let numerator = -x - &a;
        let denominator = x * &u;
        let value2 = (numerator * &denominator.inverse().unwrap()).sqrt();

        let mut recovered_value = None;

        if let Some(value) = value1 {
            if fq_element == value {
                println!("SUCCESS 1");
                recovered_value = Some(value);
            } else if fq_element == -value {
                println!("SUCCESS 2");
                recovered_value = Some(-value);
            }
        }

        if let Some(value) = value2 {
            if fq_element == value {
                println!("SUCCESS 3");
                recovered_value = Some(value)
            } else if fq_element == -value {
                println!("SUCCESS 4");
                recovered_value = Some(-value);
            }
        }

        if recovered_value.is_none() {
            println!("FAILED");
            panic!()
        }

        recovered_value.unwrap()
    };

    let fr_element_reconstructed = {
        let output = Fr::from_random_bytes(&to_bytes![fq_element_reconstructed].unwrap());
        assert!(output.is_some());
        output.unwrap()
    };

    assert_eq!(fr_element, fr_element_reconstructed);
}
