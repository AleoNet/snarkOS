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
    algorithms::signature::{SchnorrParametersGadget, SchnorrPublicKeyGadget, SchnorrPublicKeyRandomizationGadget},
    curves::edwards_bls12::EdwardsBlsGadget,
};
use snarkos_curves::{bls12_377::Fr, edwards_bls12::EdwardsAffine};
use snarkos_models::{
    algorithms::SignatureScheme,
    curves::Group,
    gadgets::{
        algorithms::SignaturePublicKeyRandomizationGadget,
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, eq::EqGadget, uint::UInt8},
    },
};
use snarkos_utilities::{bytes::ToBytes, rand::UniformRand, to_bytes};
use snarkvm_algorithms::signature::SchnorrSignature;

use blake2::Blake2s;
use rand::{thread_rng, Rng};

#[test]
fn test_schnorr_signature_randomize_public_key_gadget() {
    type Schnorr = SchnorrSignature<EdwardsAffine, Blake2s>;

    // Setup environment

    let mut cs = TestConstraintSystem::<Fr>::new();
    let rng = &mut thread_rng();

    // Native Schnorr message

    let mut message = [0u8; 32];
    rng.fill(&mut message);

    // Native Schnorr signing

    let schnorr_signature = Schnorr::setup::<_>(rng).unwrap();
    let private_key = schnorr_signature.generate_private_key(rng).unwrap();
    let public_key = schnorr_signature.generate_public_key(&private_key).unwrap();
    let signature = schnorr_signature.sign(&private_key, &message, rng).unwrap();
    assert!(schnorr_signature.verify(&public_key, &message, &signature).unwrap());

    // Native Schnorr randomization

    let random_scalar = to_bytes!(<EdwardsAffine as Group>::ScalarField::rand(rng)).unwrap();
    let randomized_public_key = schnorr_signature
        .randomize_public_key(&public_key, &random_scalar)
        .unwrap();
    let randomized_signature = schnorr_signature
        .randomize_signature(&signature, &random_scalar)
        .unwrap();
    assert!(
        schnorr_signature
            .verify(&randomized_public_key, &message, &randomized_signature)
            .unwrap()
    );

    // Circuit Schnorr randomized public key (candidate)

    let candidate_parameters_gadget = SchnorrParametersGadget::<EdwardsAffine, Fr, Blake2s>::alloc_input(
        &mut cs.ns(|| "candidate_parameters"),
        || Ok(schnorr_signature.parameters()),
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
