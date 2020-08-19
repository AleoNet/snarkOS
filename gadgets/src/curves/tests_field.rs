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

use snarkos_models::{
    curves::Field,
    gadgets::{
        curves::FieldGadget,
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, boolean::Boolean},
    },
};
use snarkos_utilities::{bititerator::BitIterator, rand::UniformRand};

use rand::{self, thread_rng, SeedableRng};
use rand_xorshift::XorShiftRng;

fn field_test<NativeF: Field, F: Field, FG: FieldGadget<NativeF, F>, CS: ConstraintSystem<F>>(
    mut cs: CS,
    a: FG,
    b: FG,
) {
    let a_native = a.get_value().unwrap();
    let b_native = b.get_value().unwrap();

    let zero = FG::zero(cs.ns(|| "zero")).unwrap();
    let zero_native = zero.get_value().unwrap();
    zero.enforce_equal(&mut cs.ns(|| "zero_equals?"), &zero).unwrap();
    assert_eq!(zero, zero);

    let one = FG::one(cs.ns(|| "one")).unwrap();
    let one_native = one.get_value().unwrap();
    assert_eq!(one, one);
    one.enforce_equal(&mut cs.ns(|| "one_equals?"), &one).unwrap();
    assert_ne!(one, zero);

    let one_dup = zero.add(cs.ns(|| "zero_plus_one"), &one).unwrap();
    one_dup
        .enforce_equal(&mut cs.ns(|| "one_plus_zero_equals"), &one)
        .unwrap();
    assert_eq!(one_dup, one);

    let two = one.add(cs.ns(|| "one_plus_one"), &one).unwrap();
    two.enforce_equal(&mut cs.ns(|| "two_equals?"), &two).unwrap();
    assert_eq!(two, two);
    assert_ne!(zero, two);
    assert_ne!(one, two);

    // a == a
    assert_eq!(a, a);

    // a + 0 = a
    let a_plus_zero = a.add(cs.ns(|| "a_plus_zero"), &zero).unwrap();
    assert_eq!(a_plus_zero, a);
    assert_eq!(a_plus_zero.get_value().unwrap(), a_native);
    a_plus_zero
        .enforce_equal(&mut cs.ns(|| "a_plus_zero_equals?"), &a)
        .unwrap();

    // a - 0 = a
    let a_minus_zero = a.sub(cs.ns(|| "a_minus_zero"), &zero).unwrap();
    assert_eq!(a_minus_zero, a);
    assert_eq!(a_minus_zero.get_value().unwrap(), a_native);
    a_minus_zero
        .enforce_equal(&mut cs.ns(|| "a_minus_zero_equals?"), &a)
        .unwrap();

    // a - a = 0
    let a_minus_a = a.sub(cs.ns(|| "a_minus_a"), &a).unwrap();
    assert_eq!(a_minus_a, zero);
    assert_eq!(a_minus_a.get_value().unwrap(), zero_native);
    a_minus_a
        .enforce_equal(&mut cs.ns(|| "a_minus_a_equals?"), &zero)
        .unwrap();

    // a + b = b + a
    let a_b = a.add(cs.ns(|| "a_plus_b"), &b).unwrap();
    let b_a = b.add(cs.ns(|| "b_plus_a"), &a).unwrap();
    assert_eq!(a_b, b_a);
    assert_eq!(a_b.get_value().unwrap(), a_native + &b_native);
    a_b.enforce_equal(&mut cs.ns(|| "a+b == b+a"), &b_a).unwrap();

    // (a + b) + a = a + (b + a)
    let ab_a = a_b.add(cs.ns(|| "a_b_plus_a"), &a).unwrap();
    let a_ba = a.add(cs.ns(|| "a_plus_b_a"), &b_a).unwrap();
    assert_eq!(ab_a, a_ba);
    assert_eq!(ab_a.get_value().unwrap(), a_native + &b_native + &a_native);
    ab_a.enforce_equal(&mut cs.ns(|| "a+b + a == a+ b+a"), &a_ba).unwrap();

    let b_times_a_plus_b = a_b.mul(cs.ns(|| "b * (a + b)"), &b).unwrap();
    let b_times_b_plus_a = b_a.mul(cs.ns(|| "b * (b + a)"), &b).unwrap();
    assert_eq!(b_times_b_plus_a, b_times_a_plus_b);
    assert_eq!(
        b_times_a_plus_b.get_value().unwrap(),
        b_native * &(b_native + &a_native)
    );
    assert_eq!(
        b_times_a_plus_b.get_value().unwrap(),
        (b_native + &a_native) * &b_native
    );
    assert_eq!(
        b_times_a_plus_b.get_value().unwrap(),
        (a_native + &b_native) * &b_native
    );
    b_times_b_plus_a
        .enforce_equal(&mut cs.ns(|| "b*(a+b) == b * (b+a)"), &b_times_a_plus_b)
        .unwrap();

    // a * 0 = 0
    assert_eq!(a.mul(cs.ns(|| "a_times_zero"), &zero).unwrap(), zero);

    // a * 1 = a
    assert_eq!(a.mul(cs.ns(|| "a_times_one"), &one).unwrap(), a);
    assert_eq!(
        a.mul(cs.ns(|| "a_times_one2"), &one).unwrap().get_value().unwrap(),
        a_native * &one_native
    );

    // a * b = b * a
    let ab = a.mul(cs.ns(|| "a_times_b"), &b).unwrap();
    let ba = b.mul(cs.ns(|| "b_times_a"), &a).unwrap();
    assert_eq!(ab, ba);
    assert_eq!(ab.get_value().unwrap(), a_native * &b_native);

    // (a * b) * a = a * (b * a)
    let ab_a = ab.mul(cs.ns(|| "ab_times_a"), &a).unwrap();
    let a_ba = a.mul(cs.ns(|| "a_times_ba"), &ba).unwrap();
    assert_eq!(ab_a, a_ba);
    assert_eq!(ab_a.get_value().unwrap(), a_native * &b_native * &a_native);

    let aa = a.mul(cs.ns(|| "a * a"), &a).unwrap();
    let a_squared = a.square(cs.ns(|| "a^2")).unwrap();
    a_squared.enforce_equal(&mut cs.ns(|| "a^2 == a*a"), &aa).unwrap();
    assert_eq!(aa, a_squared);
    assert_eq!(aa.get_value().unwrap(), a_native.square());

    let aa = a
        .mul_by_constant(cs.ns(|| "a * a via mul_by_const"), &a.get_value().unwrap())
        .unwrap();
    a_squared
        .enforce_equal(&mut cs.ns(|| "a^2 == a*a via mul_by_const"), &aa)
        .unwrap();
    assert_eq!(aa, a_squared);
    assert_eq!(aa.get_value().unwrap(), a_native.square());

    let a_b2 = a
        .add_constant(cs.ns(|| "a + b via add_const"), &b.get_value().unwrap())
        .unwrap();
    a_b.enforce_equal(&mut cs.ns(|| "a + b == a + b via add_const"), &a_b2)
        .unwrap();
    assert_eq!(a_b, a_b2);

    let a_inv = a.inverse(cs.ns(|| "a_inv")).unwrap();
    a_inv.mul_equals(cs.ns(|| "check_equals"), &a, &one).unwrap();
    assert_eq!(a_inv.get_value().unwrap(), a.get_value().unwrap().inverse().unwrap());
    assert_eq!(a_inv.get_value().unwrap(), a_native.inverse().unwrap());
    // a * a * a = a^3
    let bits = BitIterator::new([0x3])
        .map(|bit| Boolean::constant(bit))
        .collect::<Vec<_>>();
    assert_eq!(
        a_native * &(a_native * &a_native),
        a.pow(cs.ns(|| "test_pow"), &bits).unwrap().get_value().unwrap()
    );

    // a * a * a = a^3
    let mut constants = [NativeF::zero(); 4];
    for c in &mut constants {
        *c = UniformRand::rand(&mut thread_rng());
        println!("Current c[i]: {:?}", c);
    }
    let bits = [Boolean::constant(false), Boolean::constant(true)];
    let lookup_result = FG::two_bit_lookup(cs.ns(|| "Lookup"), &bits, constants.as_ref()).unwrap();
    assert_eq!(lookup_result.get_value().unwrap(), constants[2]);

    let negone: NativeF = UniformRand::rand(&mut thread_rng());

    let n = FG::alloc(&mut cs.ns(|| "alloc new var"), || Ok(negone)).unwrap();
    let _ = n.to_bytes(&mut cs.ns(|| "ToBytes")).unwrap();
    let _ = n.to_bytes_strict(&mut cs.ns(|| "ToBytes Strict")).unwrap();

    let ab_false = a
        .conditionally_add_constant(
            cs.ns(|| "Add bool with coeff false"),
            &Boolean::constant(false),
            b_native,
        )
        .unwrap();
    assert_eq!(ab_false.get_value().unwrap(), a_native);
    let ab_true = a
        .conditionally_add_constant(cs.ns(|| "Add bool with coeff true"), &Boolean::constant(true), b_native)
        .unwrap();
    assert_eq!(ab_true.get_value().unwrap(), a_native + &b_native);
}

fn random_frobenius_tests<NativeF: Field, F: Field, FG: FieldGadget<NativeF, F>, CS: ConstraintSystem<F>>(
    mut cs: CS,
    maxpower: usize,
) {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    for i in 0..(maxpower + 1) {
        let mut a = NativeF::rand(&mut rng);
        let mut a_gadget = FG::alloc(cs.ns(|| format!("a_gadget_{:?}", i)), || Ok(a)).unwrap();
        a_gadget = a_gadget.frobenius_map(cs.ns(|| format!("frob_map_{}", i)), i).unwrap();
        a.frobenius_map(i);

        assert_eq!(a_gadget.get_value().unwrap(), a);
    }
}

#[test]
fn bls12_377_field_gadgets_test() {
    use crate::curves::bls12_377::{Fq12Gadget, Fq2Gadget, Fq6Gadget, FqGadget};
    use snarkos_curves::bls12_377::{Fq, Fq12, Fq2, Fq6};

    let mut cs = TestConstraintSystem::<Fq>::new();

    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    let a = FqGadget::alloc(&mut cs.ns(|| "generate_a"), || Ok(Fq::rand(&mut rng))).unwrap();
    let b = FqGadget::alloc(&mut cs.ns(|| "generate_b"), || Ok(Fq::rand(&mut rng))).unwrap();
    field_test(cs.ns(|| "test_fq"), a, b);
    if !cs.is_satisfied() {
        println!("{:?}", cs.which_is_unsatisfied().unwrap());
    }

    let c = Fq2Gadget::alloc(&mut cs.ns(|| "generate_c"), || Ok(Fq2::rand(&mut rng))).unwrap();
    let d = Fq2Gadget::alloc(&mut cs.ns(|| "generate_d"), || Ok(Fq2::rand(&mut rng))).unwrap();
    field_test(cs.ns(|| "test_fq2"), c, d);
    random_frobenius_tests::<Fq2, _, Fq2Gadget, _>(cs.ns(|| "test_frob_fq2"), 13);
    if !cs.is_satisfied() {
        println!("{:?}", cs.which_is_unsatisfied().unwrap());
    }

    let a = Fq6Gadget::alloc(&mut cs.ns(|| "generate_e"), || Ok(Fq6::rand(&mut rng))).unwrap();
    let b = Fq6Gadget::alloc(&mut cs.ns(|| "generate_f"), || Ok(Fq6::rand(&mut rng))).unwrap();
    field_test(cs.ns(|| "test_fq6"), a, b);
    random_frobenius_tests::<Fq6, _, Fq6Gadget, _>(cs.ns(|| "test_frob_fq6"), 13);
    if !cs.is_satisfied() {
        println!("{:?}", cs.which_is_unsatisfied().unwrap());
    }

    let c = Fq12Gadget::alloc(&mut cs.ns(|| "generate_g"), || Ok(Fq12::rand(&mut rng))).unwrap();
    let d = Fq12Gadget::alloc(&mut cs.ns(|| "generate_h"), || Ok(Fq12::rand(&mut rng))).unwrap();
    field_test(cs.ns(|| "test_fq12"), c, d);
    random_frobenius_tests::<Fq12, _, Fq12Gadget, _>(cs.ns(|| "test_frob_fq12"), 13);
    if !cs.is_satisfied() {
        println!("{:?}", cs.which_is_unsatisfied().unwrap());
    }

    assert!(cs.is_satisfied());
}

#[test]
fn edwards_field_gadgets_test() {
    use crate::curves::edwards_bls12::FqGadget;
    use snarkos_curves::edwards_bls12::Fq;

    let mut cs = TestConstraintSystem::<Fq>::new();

    let mut rng = thread_rng();

    let a = FqGadget::alloc(&mut cs.ns(|| "generate_a"), || Ok(Fq::rand(&mut rng))).unwrap();
    let b = FqGadget::alloc(&mut cs.ns(|| "generate_b"), || Ok(Fq::rand(&mut rng))).unwrap();
    field_test(cs.ns(|| "test_fq"), a, b);
    if !cs.is_satisfied() {
        println!("{:?}", cs.which_is_unsatisfied().unwrap());
    }
    assert!(cs.is_satisfied());
}
