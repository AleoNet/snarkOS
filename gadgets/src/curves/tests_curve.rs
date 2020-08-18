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

use crate::curves::{
    bls12_377::{G1Gadget, G1PreparedGadget, G2Gadget, G2PreparedGadget},
    templates::bls12::Bls12PairingGadget,
};
use snarkos_curves::bls12_377::{Bls12_377, Fq, Fr, G1Projective, G2Projective};
use snarkos_models::{
    curves::{Field, One, PairingEngine, PrimeField, ProjectiveCurve},
    gadgets::{
        curves::{FieldGadget, PairingGadget},
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, boolean::Boolean, eq::EqGadget},
    },
};
use snarkos_utilities::bititerator::BitIterator;

use std::ops::Mul;

#[test]
fn bls12_377_gadget_bilinearity_test() {
    let mut cs = TestConstraintSystem::<Fq>::new();

    // let a: G1Projective = rand::random();
    // let b: G2Projective = rand::random();
    // let s: Fr = rand::random();

    let a: G1Projective = G1Projective::prime_subgroup_generator();
    let b: G2Projective = G2Projective::prime_subgroup_generator();
    let s: Fr = Fr::one() + &Fr::one();

    let sa = a.mul(&s);
    let sb = b.mul(&s);

    let a_g = G1Gadget::alloc(&mut cs.ns(|| "a"), || Ok(a)).unwrap();
    let b_g = G2Gadget::alloc(&mut cs.ns(|| "b"), || Ok(b)).unwrap();
    let sa_g = G1Gadget::alloc(&mut cs.ns(|| "sa"), || Ok(sa)).unwrap();
    let sb_g = G2Gadget::alloc(&mut cs.ns(|| "sb"), || Ok(sb)).unwrap();

    let a_prep_g = G1PreparedGadget::from_affine(&mut cs.ns(|| "a_prep"), &a_g).unwrap();
    let b_prep_g = G2PreparedGadget::from_affine(&mut cs.ns(|| "b_prep"), &b_g).unwrap();

    let sa_prep_g = G1PreparedGadget::from_affine(&mut cs.ns(|| "sa_prep"), &sa_g).unwrap();
    let sb_prep_g = G2PreparedGadget::from_affine(&mut cs.ns(|| "sb_prep"), &sb_g).unwrap();

    let (ans1_g, ans1_n) = {
        let ans_g = Bls12PairingGadget::pairing(cs.ns(|| "pair(sa, b)"), sa_prep_g.clone(), b_prep_g.clone()).unwrap();
        let ans_n = Bls12_377::pairing(sa, b);
        (ans_g, ans_n)
    };

    let (ans2_g, ans2_n) = {
        let ans_g = Bls12PairingGadget::pairing(cs.ns(|| "pair(a, sb)"), a_prep_g.clone(), sb_prep_g.clone()).unwrap();
        let ans_n = Bls12_377::pairing(a, sb);
        (ans_g, ans_n)
    };

    let (ans3_g, ans3_n) = {
        let s_iter = BitIterator::new(s.into_repr())
            .map(|bit| Boolean::constant(bit))
            .collect::<Vec<_>>();

        let mut ans_g =
            Bls12PairingGadget::pairing(cs.ns(|| "pair(a, b)"), a_prep_g.clone(), b_prep_g.clone()).unwrap();
        let mut ans_n = Bls12_377::pairing(a, b);
        ans_n = ans_n.pow(s.into_repr());
        ans_g = ans_g.pow(cs.ns(|| "pow"), &s_iter).unwrap();

        (ans_g, ans_n)
    };

    assert_eq!(ans1_n, ans2_n, "Failed ans1_native == ans2_native");
    assert_eq!(ans2_n, ans3_n, "Failed ans2_native == ans3_native");
    assert_eq!(ans1_g.get_value(), ans3_g.get_value(), "Failed ans1 == ans3");
    assert_eq!(ans1_g.get_value(), ans2_g.get_value(), "Failed ans1 == ans2");
    assert_eq!(ans2_g.get_value(), ans3_g.get_value(), "Failed ans2 == ans3");

    ans1_g.enforce_equal(&mut cs.ns(|| "ans1 == ans2?"), &ans2_g).unwrap();
    ans2_g.enforce_equal(&mut cs.ns(|| "ans2 == ans3?"), &ans3_g).unwrap();

    assert_eq!(ans1_g.get_value().unwrap(), ans1_n, "Failed native test 1");
    assert_eq!(ans2_g.get_value().unwrap(), ans2_n, "Failed native test 2");
    assert_eq!(ans3_g.get_value().unwrap(), ans3_n, "Failed native test 3");

    if !cs.is_satisfied() {
        println!("Unsatisfied: {:?}", cs.which_is_unsatisfied());
    }

    assert!(cs.is_satisfied(), "cs is not satisfied");
}
