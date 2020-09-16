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
    record::tests::{TEST_ENCRYPTED_RECORD, TEST_PRIVATE_KEY, TEST_RECORD, TEST_SERIAL_NUMBER},
    wasm::{Account, Record, SignatureScheme, SignatureSchemePublicKey, ViewKey},
};

use wasm_bindgen_test::*;

// Account Tests

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn";
    let given_address = "aleo1faksgtpmculyzt6tgaq26fe4fgdjtwualyljjvfn2q6k42ydegzspfz9uh";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}

#[wasm_bindgen_test]
pub fn view_key_from_private_key_test() {
    let given_private_key = "APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn";
    let given_view_key = "AViewKey1m8gvywHKHKfUzZiLiLoHedcdHEjKwo5TWo6efz8gK7wF";

    let view_key = ViewKey::from_private_key(given_private_key);

    println!("{} == {}", given_view_key, view_key.view_key.to_string());
    assert_eq!(given_view_key, view_key.view_key.to_string());
}

// Record Tests

#[wasm_bindgen_test]
pub fn record_decrpyption_test() {
    let view_key = ViewKey::from_private_key(TEST_PRIVATE_KEY).view_key.to_string();

    let record = Record::decrypt_record(TEST_ENCRYPTED_RECORD, &view_key);

    println!("{} == {}", TEST_RECORD, record.record.to_string());
    assert_eq!(TEST_RECORD, record.record.to_string());
}

#[wasm_bindgen_test]
pub fn serial_number_derivation_test() {
    let account = Account::from_private_key(TEST_PRIVATE_KEY);
    let private_key = account.private_key.to_string();

    let record = Record::from_string(TEST_RECORD);
    let serial_number = record.derive_serial_number(&private_key);

    println!("{} == {}", TEST_SERIAL_NUMBER, serial_number);
    assert_eq!(TEST_SERIAL_NUMBER, serial_number);
}

// Signature Tests

#[wasm_bindgen_test]
pub fn signature_public_key_from_private_key_test() {
    let given_private_key = "APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn";
    let given_public_key = "17e858cfba9f42335bd7d4751f9284671f913d841325ce548f98ae46d480211038530919083215e5376a472a61eefad25b545d3b75d43c8e2f8f821a17500103";

    let public_key = SignatureSchemePublicKey::from_private_key(given_private_key);

    println!("{} == {}", given_public_key, public_key.public_key.to_string());
    assert_eq!(given_public_key, public_key.public_key.to_string());
}

#[wasm_bindgen_test]
pub fn signature_verification_test() {
    let given_private_key = "APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn";
    let given_message = "test message";

    let signature = SignatureScheme::sign(given_private_key, given_message);
    let public_key = SignatureSchemePublicKey::from_private_key(given_private_key);

    let signature_verification = signature.verify(&public_key.public_key.to_string(), given_message);

    println!("{} == {}", true, signature_verification);
    assert!(signature_verification);
}
