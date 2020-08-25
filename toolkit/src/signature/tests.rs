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
    account::PrivateKey,
    signature::{Signature, SignaturePublicKey},
};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::str::FromStr;

#[test]
pub fn signature_test() {
    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
    let private_key = PrivateKey::new(rng);
    assert!(private_key.is_ok());

    let message: [u8; 32] = rng.gen();

    let signature = Signature::sign(&private_key.unwrap(), &message, rng);
    assert!(signature.is_ok());

    let expected_signature = "41fdc76a826b157b895012fc0bd840b65eaec5b69e9d33141960ee61b0ccdd00d0f3be67419c660afed7cd807a94396ff93864fb149c0a39148036da8c9eaa02";
    let candidate_signature = signature.unwrap().to_string();

    println!("{} == {}", expected_signature, candidate_signature);
    assert_eq!(expected_signature, candidate_signature);
}

#[test]
pub fn public_key_test() {
    let private_key = PrivateKey::from_str("APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn").unwrap();
    let public_key = SignaturePublicKey::from(&private_key);
    assert!(public_key.is_ok());

    let expected_public_key = "17e858cfba9f42335bd7d4751f9284671f913d841325ce548f98ae46d480211038530919083215e5376a472a61eefad25b545d3b75d43c8e2f8f821a17500103";
    let candidate_public_key = public_key.unwrap().to_string();

    println!("{} == {}", expected_public_key, candidate_public_key);
    assert_eq!(expected_public_key, candidate_public_key);
}

#[test]
pub fn signature_verification_test() {
    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);

    let message: [u8; 32] = rng.gen();

    let private_key = PrivateKey::from_str("APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn").unwrap();
    let public_key = SignaturePublicKey::from(&private_key);
    assert!(public_key.is_ok());

    let signature = Signature::sign(&private_key, &message, rng);
    assert!(signature.is_ok());

    let verification = signature.unwrap().verify(&public_key.unwrap(), &message);
    assert!(verification.is_ok());
    assert!(verification.unwrap())
}
