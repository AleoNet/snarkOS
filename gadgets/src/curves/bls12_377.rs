use crate::curves::templates::bls12::{
    Bls12PairingGadget,
    G1Gadget as Bls12G1Gadget,
    G1PreparedGadget as Bls12G1PreparedGadget,
    G2Gadget as Bls12G2Gadget,
    G2PreparedGadget as Bls12G2PreparedGadget,
};
use snarkos_curves::bls12_377::{Bls12_377Parameters, Fq, Fq12Parameters, Fq2Parameters, Fq6Parameters};
use snarkos_models::gadgets::curves::{Fp12Gadget, Fp2Gadget, Fp6Gadget, FpGadget};

pub type FqGadget = FpGadget<Fq>;
pub type Fq2Gadget = Fp2Gadget<Fq2Parameters, Fq>;
pub type Fq6Gadget = Fp6Gadget<Fq6Parameters, Fq>;
pub type Fq12Gadget = Fp12Gadget<Fq12Parameters, Fq>;
pub type G1Gadget = Bls12G1Gadget<Bls12_377Parameters>;
pub type G1PreparedGadget = Bls12G1PreparedGadget<Bls12_377Parameters>;
pub type G2Gadget = Bls12G2Gadget<Bls12_377Parameters>;
pub type G2PreparedGadget = Bls12G2PreparedGadget<Bls12_377Parameters>;
pub type PairingGadget = Bls12PairingGadget<Bls12_377Parameters>;

#[cfg(test)]
mod test {
    use super::*;
    use snarkos_curves::bls12_377::{Fq, Fr, G1Projective as G1, G2Projective as G2};
    use snarkos_models::{
        curves::{AffineCurve, PrimeField, ProjectiveCurve},
        gadgets::{
            curves::{FieldGadget, GroupGadget},
            r1cs::{ConstraintSystem, TestConstraintSystem},
            utilities::{
                alloc::AllocGadget,
                boolean::{AllocatedBit, Boolean},
                eq::EqGadget,
                select::CondSelectGadget,
            },
        },
    };
    use snarkvm_utilities::{bititerator::BitIterator, rand::UniformRand};

    use rand::{self, SeedableRng};
    use rand_xorshift::XorShiftRng;

    #[test]
    fn bls12_g1_constraint_costs() {
        let mut cs = TestConstraintSystem::<Fq>::new();

        let bit = AllocatedBit::alloc(&mut cs.ns(|| "bool"), || Ok(true)).unwrap().into();

        let a: G1 = rand::random();
        let b: G1 = rand::random();
        let gadget_a = G1Gadget::alloc(&mut cs.ns(|| "a"), || Ok(a)).unwrap();
        let gadget_b = G1Gadget::alloc(&mut cs.ns(|| "b"), || Ok(b)).unwrap();
        let alloc_cost = cs.num_constraints();
        let _ = G1Gadget::conditionally_select(&mut cs.ns(|| "cond_select"), &bit, &gadget_a, &gadget_b).unwrap();
        let cond_select_cost = cs.num_constraints() - alloc_cost;

        let _ = gadget_a.add(&mut cs.ns(|| "ab"), &gadget_b).unwrap();
        let add_cost = cs.num_constraints() - cond_select_cost - alloc_cost;

        assert!(cs.is_satisfied());
        assert_eq!(cond_select_cost, <G1Gadget as CondSelectGadget<Fq>>::cost());
        assert_eq!(add_cost, G1Gadget::cost_of_add());
    }

    #[test]
    fn bls12_g2_constraint_costs() {
        let mut cs = TestConstraintSystem::<Fq>::new();

        let bit = AllocatedBit::alloc(&mut cs.ns(|| "bool"), || Ok(true)).unwrap().into();

        let a: G2 = rand::random();
        let b: G2 = rand::random();
        let gadget_a = G2Gadget::alloc(&mut cs.ns(|| "a"), || Ok(a)).unwrap();
        let gadget_b = G2Gadget::alloc(&mut cs.ns(|| "b"), || Ok(b)).unwrap();
        let alloc_cost = cs.num_constraints();
        let _ = G2Gadget::conditionally_select(&mut cs.ns(|| "cond_select"), &bit, &gadget_a, &gadget_b).unwrap();
        let cond_select_cost = cs.num_constraints() - alloc_cost;

        let _ = gadget_a.add(&mut cs.ns(|| "ab"), &gadget_b).unwrap();
        let add_cost = cs.num_constraints() - cond_select_cost - alloc_cost;

        assert!(cs.is_satisfied());
        assert_eq!(cond_select_cost, <G2Gadget as CondSelectGadget<Fq>>::cost());
        assert_eq!(add_cost, G2Gadget::cost_of_add());
    }

    #[test]
    fn bls12_g1_gadget_test() {
        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let mut cs = TestConstraintSystem::<Fq>::new();

        let a = G1::rand(&mut rng);
        let b = G1::rand(&mut rng);
        let a_affine = a.into_affine();
        let b_affine = b.into_affine();
        let mut gadget_a = G1Gadget::alloc(&mut cs.ns(|| "a"), || Ok(a)).unwrap();
        let gadget_b = G1Gadget::alloc(&mut cs.ns(|| "b"), || Ok(b)).unwrap();
        assert_eq!(gadget_a.x.value.unwrap(), a_affine.x);
        assert_eq!(gadget_a.y.value.unwrap(), a_affine.y);
        assert_eq!(gadget_b.x.value.unwrap(), b_affine.x);
        assert_eq!(gadget_b.y.value.unwrap(), b_affine.y);

        // Check addition
        let ab = a + &b;
        let ab_affine = ab.into_affine();
        let gadget_ab = gadget_a.add(&mut cs.ns(|| "ab"), &gadget_b).unwrap();
        let gadget_ba = gadget_b.add(&mut cs.ns(|| "ba"), &gadget_a).unwrap();
        gadget_ba
            .enforce_equal(&mut cs.ns(|| "b + a == a + b?"), &gadget_ab)
            .unwrap();

        let ab_val = gadget_ab
            .get_value()
            .expect("Doubling should be successful")
            .into_affine();
        assert_eq!(ab_val, ab_affine, "Result of addition is unequal");

        // Check doubling
        let aa = a.double();
        let aa_affine = aa.into_affine();
        gadget_a.double_in_place(&mut cs.ns(|| "2a")).unwrap();
        let aa_val = gadget_a
            .get_value()
            .expect("Doubling should be successful")
            .into_affine();
        assert_eq!(aa_val, aa_affine, "Gadget and native values are unequal after double.");

        // Check mul_bits
        let scalar = Fr::rand(&mut rng);
        let native_result = aa.into_affine().mul(scalar) + &b;
        let native_result = native_result.into_affine();

        let mut scalar: Vec<bool> = BitIterator::new(scalar.into_repr()).collect();
        // Get the scalar bits into little-endian form.
        scalar.reverse();
        let input = Vec::<Boolean>::alloc(cs.ns(|| "Input"), || Ok(scalar)).unwrap();
        let result = gadget_a
            .mul_bits(cs.ns(|| "mul_bits"), &gadget_b, input.iter())
            .unwrap();
        let result_val = result.get_value().unwrap().into_affine();
        assert_eq!(
            result_val, native_result,
            "gadget & native values are diff. after scalar mul"
        );

        if !cs.is_satisfied() {
            println!("{:?}", cs.which_is_unsatisfied().unwrap());
        }

        assert!(cs.is_satisfied());
    }

    #[test]
    fn bls12_g2_gadget_test() {
        let mut cs = TestConstraintSystem::<Fq>::new();

        let a: G2 = rand::random();
        let b: G2 = rand::random();
        let a_affine = a.into_affine();
        let b_affine = b.into_affine();

        let mut gadget_a = G2Gadget::alloc(&mut cs.ns(|| "a"), || Ok(a)).unwrap();
        let gadget_b = G2Gadget::alloc(&mut cs.ns(|| "b"), || Ok(b)).unwrap();
        assert_eq!(gadget_a.x.get_value().unwrap(), a_affine.x);
        assert_eq!(gadget_a.y.get_value().unwrap(), a_affine.y);
        assert_eq!(gadget_b.x.get_value().unwrap(), b_affine.x);
        assert_eq!(gadget_b.y.get_value().unwrap(), b_affine.y);

        let ab = a + &b;
        let ab_affine = ab.into_affine();
        let gadget_ab = gadget_a.add(&mut cs.ns(|| "ab"), &gadget_b).unwrap();
        let gadget_ba = gadget_b.add(&mut cs.ns(|| "ba"), &gadget_a).unwrap();
        gadget_ba
            .enforce_equal(&mut cs.ns(|| "b + a == a + b?"), &gadget_ab)
            .unwrap();
        assert_eq!(gadget_ab.x.get_value().unwrap(), ab_affine.x);
        assert_eq!(gadget_ab.y.get_value().unwrap(), ab_affine.y);

        let aa = a.double();
        let aa_affine = aa.into_affine();
        gadget_a.double_in_place(&mut cs.ns(|| "2a")).unwrap();

        assert_eq!(gadget_a.x.get_value().unwrap(), aa_affine.x);
        assert_eq!(gadget_a.y.get_value().unwrap(), aa_affine.y);

        if !cs.is_satisfied() {
            println!("{:?}", cs.which_is_unsatisfied().unwrap());
        }

        assert!(cs.is_satisfied());
    }
}
