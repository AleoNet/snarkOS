use crate::curves::tests_group::group_test;
use snarkos_curves::templates::twisted_edwards_extended::GroupAffine as TEAffine;
use snarkos_models::{
    curves::{Field, Group, PrimeField, TEModelParameters},
    gadgets::{
        curves::GroupGadget,
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            boolean::{AllocatedBit, Boolean},
            select::CondSelectGadget,
        },
    },
};
use snarkos_utilities::{bititerator::BitIterator, rand::UniformRand};

use rand::thread_rng;

pub(crate) fn edwards_test<F, P, GG, CS>(cs: &mut CS)
where
    F: Field,
    P: TEModelParameters,
    GG: GroupGadget<TEAffine<P>, F, Value = TEAffine<P>>,
    CS: ConstraintSystem<F>,
{
    let a: TEAffine<P> = UniformRand::rand(&mut thread_rng());
    let b: TEAffine<P> = UniformRand::rand(&mut thread_rng());
    let gadget_a = GG::alloc(&mut cs.ns(|| "a"), || Ok(a)).unwrap();
    let gadget_b = GG::alloc(&mut cs.ns(|| "b"), || Ok(b)).unwrap();
    assert_eq!(gadget_a.get_value().unwrap(), a);
    assert_eq!(gadget_b.get_value().unwrap(), b);
    group_test::<F, TEAffine<P>, GG, _>(&mut cs.ns(|| "GroupTest(a, b)"), gadget_a.clone(), gadget_b);

    // Check mul_bits
    let scalar: <TEAffine<P> as Group>::ScalarField = UniformRand::rand(&mut thread_rng());
    let native_result = a.mul(&scalar);

    let mut scalar: Vec<bool> = BitIterator::new(scalar.into_repr()).collect();
    // Get the scalar bits into little-endian form.
    scalar.reverse();
    let input = Vec::<Boolean>::alloc(cs.ns(|| "Input"), || Ok(scalar)).unwrap();
    let zero = GG::zero(cs.ns(|| "zero")).unwrap();
    let result = gadget_a.mul_bits(cs.ns(|| "mul_bits"), &zero, input.iter()).unwrap();
    let gadget_value = result.get_value().expect("Gadget_result failed");
    assert_eq!(native_result, gadget_value);
}

pub(crate) fn edwards_constraint_costs<F, P, GG, CS>(cs: &mut CS)
where
    F: Field,
    P: TEModelParameters,
    GG: GroupGadget<TEAffine<P>, F, Value = TEAffine<P>>,
    CS: ConstraintSystem<F>,
{
    let bit = AllocatedBit::alloc(&mut cs.ns(|| "bool"), || Ok(true)).unwrap().into();

    let a: TEAffine<P> = rand::random();
    let b: TEAffine<P> = rand::random();
    let gadget_a = GG::alloc(&mut cs.ns(|| "a"), || Ok(a)).unwrap();
    let gadget_b = GG::alloc(&mut cs.ns(|| "b"), || Ok(b)).unwrap();
    let alloc_cost = cs.num_constraints();
    let _ = GG::conditionally_select(&mut cs.ns(|| "cond_select"), &bit, &gadget_a, &gadget_b).unwrap();
    let cond_select_cost = cs.num_constraints() - alloc_cost;

    let _ = gadget_a.add(&mut cs.ns(|| "ab"), &gadget_b).unwrap();
    let add_cost = cs.num_constraints() - cond_select_cost - alloc_cost;
    assert_eq!(cond_select_cost, <GG as CondSelectGadget<_>>::cost());
    assert_eq!(add_cost, GG::cost_of_add());
}
