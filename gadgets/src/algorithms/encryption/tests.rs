use crate::{algorithms::encryption::*, curves::edwards_bls12::EdwardsBlsGadget};
use snarkos_algorithms::encryption::GroupEncryption;
use snarkos_curves::{bls12_377::Fr, edwards_bls12::EdwardsProjective};
use snarkos_models::{
    algorithms::EncryptionScheme,
    curves::{Group, ProjectiveCurve},
    gadgets::{
        algorithms::EncryptionGadget,
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, eq::EqGadget},
    },
};
//use snarkos_utilities::{bytes::ToBytes, rand::UniformRand, to_bytes};

use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

type TestEncryptionScheme = GroupEncryption<EdwardsProjective>;
type TestEncryptionSchemeGadget = GroupEncryptionGadget<EdwardsProjective, Fr, EdwardsBlsGadget>;

pub fn generate_input<G: Group + ProjectiveCurve, R: Rng>(input_size: usize, rng: &mut R) -> Vec<G> {
    let mut input = vec![];
    for _ in 0..input_size {
        input.push(G::rand(rng))
    }

    input
}

#[test]
fn test_group_encryption_public_key_gadget() {
    let mut cs = TestConstraintSystem::<Fr>::new();
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    let encryption_scheme = TestEncryptionScheme::setup(rng);

    let private_key = encryption_scheme.generate_private_key(rng);
    let public_key = encryption_scheme.generate_public_key(&private_key);

    let parameters_gadget = <TestEncryptionSchemeGadget as EncryptionGadget<_, _>>::ParametersGadget::alloc(
        &mut cs.ns(|| "parameters_gadget"),
        || Ok(&encryption_scheme.parameters),
    )
    .unwrap();
    let private_key_gadget = <TestEncryptionSchemeGadget as EncryptionGadget<_, _>>::PrivateKeyGadget::alloc(
        &mut cs.ns(|| "private_key_gadget"),
        || Ok(&private_key),
    )
    .unwrap();
    let expected_public_key_gadget = <TestEncryptionSchemeGadget as EncryptionGadget<_, _>>::PublicKeyGadget::alloc(
        &mut cs.ns(|| "public_key_gadget"),
        || Ok(&public_key),
    )
    .unwrap();

    let public_key_gadget = TestEncryptionSchemeGadget::check_public_key_gadget(
        &mut cs.ns(|| "public_key_gadget_evaluation"),
        &parameters_gadget,
        &private_key_gadget,
    )
    .unwrap();

    expected_public_key_gadget
        .enforce_equal(
            cs.ns(|| "Check that declared and computed public keys are equal"),
            &public_key_gadget,
        )
        .unwrap();

    if !cs.is_satisfied() {
        println!("which is unsatisfied: {:?}", cs.which_is_unsatisfied().unwrap());
    }
    assert!(cs.is_satisfied());
}
