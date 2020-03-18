use crate::{
    algorithms::signature::{SchnorrParametersGadget, SchnorrPublicKeyGadget, SchnorrPublicKeyRandomizationGadget},
    curves::edwards_bls12::EdwardsBlsGadget,
};
use snarkos_algorithms::signature::SchnorrSignature;
use snarkos_curves::{bls12_377::Fr, edwards_bls12::EdwardsAffine};
use snarkos_models::{
    algorithms::SignatureScheme,
    curves::Group,
    gadgets::{
        algorithms::SignaturePublicKeyRandomizationGadget,
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, eq::EqGadget, uint8::UInt8},
    },
};
use snarkos_utilities::{bytes::ToBytes, rand::UniformRand, to_bytes};

use blake2::Blake2s;
use rand::{thread_rng, Rng};

#[test]
fn test_schnorr_signature_gadget() {
    type Schnorr = SchnorrSignature<EdwardsAffine, Blake2s>;

    // Setup environment

    let mut cs = TestConstraintSystem::<Fr>::new();
    let rng = &mut thread_rng();

    // Native Schnorr message

    let mut message = [0u8; 32];
    rng.fill(&mut message);

    // Native Schnorr signing

    let parameters = Schnorr::setup::<_>(rng).unwrap();
    let (public_key, private_key) = Schnorr::keygen(&parameters, rng).unwrap();
    let signature = Schnorr::sign(&parameters, &private_key, &message, rng).unwrap();
    assert!(Schnorr::verify(&parameters, &public_key, &message, &signature).unwrap());

    // Native Schnorr randomization

    let random_scalar = to_bytes!(<EdwardsAffine as Group>::ScalarField::rand(rng)).unwrap();
    let randomized_public_key = Schnorr::randomize_public_key(&parameters, &public_key, &random_scalar).unwrap();
    let randomized_signature = Schnorr::randomize_signature(&parameters, &signature, &random_scalar).unwrap();
    assert!(Schnorr::verify(&parameters, &randomized_public_key, &message, &randomized_signature).unwrap());

    // Circuit Schnorr randomized public key (candidate)

    let candidate_parameters_gadget = SchnorrParametersGadget::<EdwardsAffine, Fr, EdwardsBlsGadget>::alloc_input(
        &mut cs.ns(|| "candidate_parameters"),
        || Ok(&parameters),
    )
    .unwrap();

    let candidate_public_key_gadget = SchnorrPublicKeyGadget::<EdwardsAffine, Fr, EdwardsBlsGadget>::alloc(
        &mut cs.ns(|| "candidate_public_key"),
        || Ok(&public_key),
    )
    .unwrap();

    let candidate_randomizer = UInt8::alloc_vec(&mut cs.ns(|| "candidate_randomizer"), &random_scalar).unwrap();

    let candidate_randomized_public_key_gadget = <SchnorrPublicKeyRandomizationGadget<
        EdwardsAffine,
        Fr,
        EdwardsBlsGadget,
    > as SignaturePublicKeyRandomizationGadget<Schnorr, Fr>>::check_randomization_gadget(
        &mut cs.ns(|| "candidate_randomized_public_key"),
        &candidate_parameters_gadget,
        &candidate_public_key_gadget,
        &candidate_randomizer,
    )
    .unwrap();

    // Circuit Schnorr randomized public key (given)

    let given_randomized_public_key_gadget =
        SchnorrPublicKeyGadget::<EdwardsAffine, Fr, EdwardsBlsGadget>::alloc_input(
            &mut cs.ns(|| "given_randomized_public_key"),
            || Ok(randomized_public_key),
        )
        .unwrap();

    candidate_randomized_public_key_gadget
        .enforce_equal(&mut cs.ns(|| "enforce_equal"), &given_randomized_public_key_gadget)
        .unwrap();

    if !cs.is_satisfied() {
        println!("which is unsatisfied: {:?}", cs.which_is_unsatisfied().unwrap());
    }
    assert!(cs.is_satisfied());
}
