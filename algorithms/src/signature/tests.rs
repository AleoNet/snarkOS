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

use crate::signature::SchnorrSignature;
use snarkos_curves::edwards_sw6::EdwardsAffine as Edwards;
use snarkos_models::{algorithms::SignatureScheme, curves::Group};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
    to_bytes,
};

use blake2::Blake2s;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

type TestSignature = SchnorrSignature<Edwards, Blake2s>;

fn sign_and_verify<S: SignatureScheme>(message: &[u8]) {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);
    let schnorr_signature = S::setup::<_>(rng).unwrap();
    let private_key = schnorr_signature.generate_private_key(rng).unwrap();
    let public_key = schnorr_signature.generate_public_key(&private_key).unwrap();
    let signature = schnorr_signature.sign(&private_key, message, rng).unwrap();
    assert!(schnorr_signature.verify(&public_key, &message, &signature).unwrap());
}

fn failed_verification<S: SignatureScheme>(message: &[u8], bad_message: &[u8]) {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);
    let schnorr_signature = S::setup::<_>(rng).unwrap();
    let private_key = schnorr_signature.generate_private_key(rng).unwrap();
    let public_key = schnorr_signature.generate_public_key(&private_key).unwrap();
    let signature = schnorr_signature.sign(&private_key, message, rng).unwrap();
    assert!(!schnorr_signature.verify(&public_key, bad_message, &signature).unwrap());
}

fn randomize_and_verify<S: SignatureScheme>(message: &[u8], randomness: &[u8]) {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);
    let schnorr_signature = S::setup::<_>(rng).unwrap();
    let private_key = schnorr_signature.generate_private_key(rng).unwrap();
    let public_key = schnorr_signature.generate_public_key(&private_key).unwrap();
    let signature = schnorr_signature.sign(&private_key, message, rng).unwrap();
    assert!(schnorr_signature.verify(&public_key, message, &signature).unwrap());

    let randomized_public_key = schnorr_signature.randomize_public_key(&public_key, randomness).unwrap();
    let randomized_signature = schnorr_signature.randomize_signature(&signature, randomness).unwrap();
    assert!(
        schnorr_signature
            .verify(&randomized_public_key, &message, &randomized_signature)
            .unwrap()
    );
}

fn signature_scheme_parameter_serialization<S: SignatureScheme>() {
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);

    let signature_scheme = S::setup(rng).unwrap();
    let signature_scheme_parameters = signature_scheme.parameters();

    let signature_scheme_parameters_bytes = to_bytes![signature_scheme_parameters].unwrap();
    let recovered_signature_scheme_parameters: <S as SignatureScheme>::Parameters =
        FromBytes::read(&signature_scheme_parameters_bytes[..]).unwrap();

    assert_eq!(signature_scheme_parameters, &recovered_signature_scheme_parameters);
}

#[test]
fn schnorr_signature_test() {
    let message = "Hi, I am a Schnorr signature!";
    let rng = &mut XorShiftRng::seed_from_u64(1231275789u64);
    sign_and_verify::<TestSignature>(message.as_bytes());
    failed_verification::<TestSignature>(message.as_bytes(), "Bad message".as_bytes());
    let random_scalar = to_bytes!(<Edwards as Group>::ScalarField::rand(rng)).unwrap();
    randomize_and_verify::<TestSignature>(message.as_bytes(), &random_scalar.as_slice());
}

#[test]
fn schnorr_signature_scheme_parameters_serialization() {
    signature_scheme_parameter_serialization::<TestSignature>();
}
