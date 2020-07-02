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

fn generate_input<G: Group + ProjectiveCurve, R: Rng>(input_size: usize, rng: &mut R) -> Vec<G> {
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

    println!("number of constraints for inputs: {}", cs.num_constraints());

    expected_public_key_gadget
        .enforce_equal(
            cs.ns(|| "Check that declared and computed public keys are equal"),
            &public_key_gadget,
        )
        .unwrap();

    println!("number of constraints total: {}", cs.num_constraints());

    if !cs.is_satisfied() {
        println!("which is unsatisfied: {:?}", cs.which_is_unsatisfied().unwrap());
    }
    assert!(cs.is_satisfied());
}

#[test]
fn test_group_encryption_gadget() {
    let mut cs = TestConstraintSystem::<Fr>::new();
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    let encryption_scheme = TestEncryptionScheme::setup(rng);

    let private_key = encryption_scheme.generate_private_key(rng);
    let public_key = encryption_scheme.generate_public_key(&private_key);

    let randomness = encryption_scheme.generate_randomness(&public_key, rng).unwrap();
    let message = generate_input(32, rng);
    let blinding_exponents = encryption_scheme
        .generate_blinding_exponents(&public_key, &randomness, message.len())
        .unwrap();
    let ciphertext = encryption_scheme.encrypt(&public_key, &randomness, &message).unwrap();

    // Alloc parameters, public key, plaintext, randomness, and blinding exponents
    let parameters_gadget = <TestEncryptionSchemeGadget as EncryptionGadget<_, _>>::ParametersGadget::alloc(
        &mut cs.ns(|| "parameters_gadget"),
        || Ok(&encryption_scheme.parameters),
    )
    .unwrap();
    let public_key_gadget = <TestEncryptionSchemeGadget as EncryptionGadget<_, _>>::PublicKeyGadget::alloc(
        &mut cs.ns(|| "public_key_gadget"),
        || Ok(&public_key),
    )
    .unwrap();
    let plaintext_gadget = <TestEncryptionSchemeGadget as EncryptionGadget<_, _>>::PlaintextGadget::alloc(
        &mut cs.ns(|| "plaintext_gadget"),
        || Ok(&message),
    )
    .unwrap();
    let randomness_gadget = <TestEncryptionSchemeGadget as EncryptionGadget<_, _>>::RandomnessGadget::alloc(
        &mut cs.ns(|| "randomness_gadget"),
        || Ok(&randomness),
    )
    .unwrap();
    let blinding_exponents_gadget =
        <TestEncryptionSchemeGadget as EncryptionGadget<_, _>>::BlindingExponentGadget::alloc(
            &mut cs.ns(|| "blinding_exponents_gadget"),
            || Ok(&blinding_exponents),
        )
        .unwrap();

    // Expected ciphertext gadget
    let expected_ciphertext_gadget = <TestEncryptionSchemeGadget as EncryptionGadget<_, _>>::CiphertextGadget::alloc(
        &mut cs.ns(|| "ciphertext_gadget"),
        || Ok(&ciphertext),
    )
    .unwrap();

    println!("number of constraints for inputs: {}", cs.num_constraints());

    let ciphertext_gadget = TestEncryptionSchemeGadget::check_encryption_gadget(
        &mut cs.ns(|| "ciphertext_gadget_evaluation"),
        &parameters_gadget,
        &randomness_gadget,
        &public_key_gadget,
        &plaintext_gadget,
        &blinding_exponents_gadget,
    )
    .unwrap();

    expected_ciphertext_gadget
        .enforce_equal(
            cs.ns(|| "Check that declared and computed ciphertexts are equal"),
            &ciphertext_gadget,
        )
        .unwrap();

    println!("number of constraints total: {}", cs.num_constraints());

    if !cs.is_satisfied() {
        println!("which is unsatisfied: {:?}", cs.which_is_unsatisfied().unwrap());
    }
    assert!(cs.is_satisfied());
}
