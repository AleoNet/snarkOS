use crate::{algorithms::signature::*, curves::edwards_bls12::EdwardsBlsGadget};
use snarkos_algorithms::signature::SchnorrSignature;
use snarkos_curves::{bls12_377::Fr, edwards_bls12::EdwardsProjective as EdwardsBls};
use snarkos_models::{
    algorithms::SignatureScheme,
    gadgets::{
        algorithms::SignaturePublicKeyRandomizationGadget,
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, eq::EqGadget, uint8::UInt8, ToBytesGadget},
    },
};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use blake2::Blake2s as Blake2sHash;

use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

pub type SGadget = SchnorrPublicKeyRandomizationGadget<EdwardsBls, Fr, EdwardsBlsGadget>;
pub type S = SchnorrSignature<EdwardsBls, Blake2sHash>;

#[test]
fn test_schnorr_signature_pk() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    let mut cs = TestConstraintSystem::<Fr>::new();

    // Setup native signature components

    let parameters = S::setup(&mut rng).unwrap();
    let (pk, _) = S::keygen(&parameters, &mut rng).unwrap();

    // Allocate Circuit Inputs

    let pk_sig = <SGadget as SignaturePublicKeyRandomizationGadget<S, _>>::PublicKeyGadget::alloc(
        &mut cs.ns(|| "Declare pk"),
        || Ok(&pk),
    )
    .unwrap();

    let pk_sig_bytes = pk_sig.to_bytes(&mut cs.ns(|| "Convert pk gadget to bytes")).unwrap();

    let direct_pk_sig_bytes = UInt8::alloc_vec(
        &mut cs.ns(|| "Declare pk directly into bytes"),
        &to_bytes![&pk].unwrap(),
    )
    .unwrap();

    // Verify native and gadget to_byte conversion is correct

    pk_sig_bytes
        .enforce_equal(
            &mut cs.ns(|| "Check that the gadget and native to_bytes are equal"),
            &direct_pk_sig_bytes,
        )
        .unwrap();

    if !cs.is_satisfied() {
        println!("which is unsatisfied: {:?}", cs.which_is_unsatisfied().unwrap());
    }
    assert!(cs.is_satisfied());

    assert_eq!(pk_sig_bytes, direct_pk_sig_bytes);
}

#[test]
fn test_schnorr_sn_generation() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    let mut cs = TestConstraintSystem::<Fr>::new();

    // Setup native signature components

    let parameters = S::setup(&mut rng).unwrap();
    let (pk, _) = S::keygen(&parameters, &mut rng).unwrap();
    //    let rpk = S::randomize_public_key(&parameters, &pk, &randomness[..]).unwrap();

    //    let sk_prf: [u8; 32] = rng.gen();
    let randomness: [u8; 32] = rng.gen();

    let sn = S::randomize_public_key(&parameters, &pk, &randomness[..]).unwrap();

    // Allocate Circuit Values

    let sig_pp = <SGadget as SignaturePublicKeyRandomizationGadget<S, _>>::ParametersGadget::alloc_input(
        &mut cs.ns(|| "Declare SIG Parameters"),
        || Ok(&parameters),
    )
    .unwrap();

    let pk_sig = <SGadget as SignaturePublicKeyRandomizationGadget<S, _>>::PublicKeyGadget::alloc(
        &mut cs.ns(|| "Declare pk"),
        || Ok(&pk),
    )
    .unwrap();

    let randomizer_bytes = UInt8::alloc_vec(&mut cs.ns(|| "declare randomness"), &randomness).unwrap();

    let candidate_sn = <SGadget as SignaturePublicKeyRandomizationGadget<S, _>>::check_randomization_gadget(
        &mut cs.ns(|| "Compute serial number"),
        &sig_pp,
        &pk_sig,
        &randomizer_bytes,
    )
    .unwrap();

    let given_sn = <SGadget as SignaturePublicKeyRandomizationGadget<S, _>>::PublicKeyGadget::alloc_input(
        &mut cs.ns(|| "Declare given serial number"),
        || Ok(sn),
    )
    .unwrap();

    candidate_sn
        .enforce_equal(
            &mut cs.ns(|| "Check that given and computed serial numbers are equal"),
            &given_sn,
        )
        .unwrap();

    if !cs.is_satisfied() {
        println!("which is unsatisfied: {:?}", cs.which_is_unsatisfied().unwrap());
    }
    assert!(cs.is_satisfied());
}
