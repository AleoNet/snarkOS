use crate::curves::edwards_bls12::EdwardsBlsGadget;
use snarkos_curves::edwards_bls12::{EdwardsProjective, Fq};
use snarkos_models::{
    curves::{Field, Group},
    gadgets::{
        curves::GroupGadget,
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::alloc::AllocGadget,
    },
};

use rand;

pub fn group_test<F: Field, G: Group, GG: GroupGadget<G, F>, CS: ConstraintSystem<F>>(cs: &mut CS, a: GG, b: GG) {
    let zero = GG::zero(cs.ns(|| "Zero")).unwrap();
    assert_eq!(zero, zero);

    // a == a
    assert_eq!(a, a);
    // a + 0 = a
    assert_eq!(a.add(cs.ns(|| "a_plus_zero"), &zero).unwrap(), a);
    // a - 0 = a
    assert_eq!(a.sub(cs.ns(|| "a_minus_zero"), &zero).unwrap(), a);
    // a - a = 0
    assert_eq!(a.sub(cs.ns(|| "a_minus_a"), &a).unwrap(), zero);
    // a + b = b + a
    let a_b = a.add(cs.ns(|| "a_plus_b"), &b).unwrap();
    let b_a = b.add(cs.ns(|| "b_plus_a"), &a).unwrap();
    assert_eq!(a_b, b_a);
    // (a + b) + a = a + (b + a)
    let ab_a = a_b.add(&mut cs.ns(|| "a_b_plus_a"), &a).unwrap();
    let a_ba = a.add(&mut cs.ns(|| "a_plus_b_a"), &b_a).unwrap();
    assert_eq!(ab_a, a_ba);
    // a.double() = a + a
    let a_a = a.add(cs.ns(|| "a + a"), &a).unwrap();
    let mut a2 = a.clone();
    a2.double_in_place(cs.ns(|| "2a")).unwrap();
    assert_eq!(a2, a_a);
    // b.double() = b + b
    let mut b2 = b.clone();
    b2.double_in_place(cs.ns(|| "2b")).unwrap();
    let b_b = b.add(cs.ns(|| "b + b"), &b).unwrap();
    assert_eq!(b2, b_b);

    let _ = a.to_bytes(&mut cs.ns(|| "ToBytes")).unwrap();
    let _ = a.to_bytes_strict(&mut cs.ns(|| "ToBytes Strict")).unwrap();

    let _ = b.to_bytes(&mut cs.ns(|| "b ToBytes")).unwrap();
    let _ = b.to_bytes_strict(&mut cs.ns(|| "b ToBytes Strict")).unwrap();
}

#[test]
fn edwards_bls12_group_gadgets_test() {
    let mut cs = TestConstraintSystem::<Fq>::new();

    let a: EdwardsProjective = rand::random();
    let b: EdwardsProjective = rand::random();

    let a = EdwardsBlsGadget::alloc(&mut cs.ns(|| "generate_a"), || Ok(a)).unwrap();
    let b = EdwardsBlsGadget::alloc(&mut cs.ns(|| "generate_b"), || Ok(b)).unwrap();
    group_test::<_, EdwardsProjective, _, _>(&mut cs.ns(|| "GroupTest(a, b)"), a, b);
}
