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
    wasm::{Account, Address, Record, ViewKey},
};

use wasm_bindgen_test::*;

// Account Tests

#[wasm_bindgen_test]
pub fn test_account_from_private_key() {
    let given_private_key = "APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn";
    let given_address = "aleo1faksgtpmculyzt6tgaq26fe4fgdjtwualyljjvfn2q6k42ydegzspfz9uh";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}

#[wasm_bindgen_test]
pub fn test_view_key_from_private_key() {
    let given_private_key = "APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn";
    let given_view_key = "AViewKey1m8gvywHKHKfUzZiLiLoHedcdHEjKwo5TWo6efz8gK7wF";

    let view_key = ViewKey::from_private_key(given_private_key);

    println!("{} == {}", given_view_key, view_key.view_key.to_string());
    assert_eq!(given_view_key, view_key.view_key.to_string());
}

// Record Tests

#[wasm_bindgen_test]
pub fn test_record_decryption() {
    let view_key = ViewKey::from_private_key(TEST_PRIVATE_KEY).view_key.to_string();

    let record = Record::decrypt(TEST_ENCRYPTED_RECORD, &view_key);

    println!("{} == {}", TEST_RECORD, record.record.to_string());
    assert_eq!(TEST_RECORD, record.record.to_string());
}

#[wasm_bindgen_test]
pub fn test_serial_number_derivation() {
    let account = Account::from_private_key(TEST_PRIVATE_KEY);
    let private_key = account.private_key.to_string();

    let record = Record::from_string(TEST_RECORD);
    let serial_number = record.to_serial_number(&private_key);

    println!("{} == {}", TEST_SERIAL_NUMBER, serial_number);
    assert_eq!(TEST_SERIAL_NUMBER, serial_number);
}

// Signature Tests

// Test Schnorr signatures where:
//   The Account Private Key `sk_sig` is the signature private key.
//   The Account Private Key `pk_sig` is the signature public key.

#[wasm_bindgen_test]
pub fn test_signature_public_key_from_private_key() {
    let given_private_key = "APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn";
    let given_public_key = "17e858cfba9f42335bd7d4751f9284671f913d841325ce548f98ae46d480211038530919083215e5376a472a61eefad25b545d3b75d43c8e2f8f821a17500103";

    let account = Account::from_private_key(given_private_key);
    let candidate_public_key = account.to_signature_public_key();

    println!("{} == {}", given_public_key, candidate_public_key);
    assert_eq!(given_public_key, candidate_public_key);
}

#[wasm_bindgen_test]
pub fn test_private_key_signature_verification() {
    let given_private_key = "APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn";
    let given_message = "test message";

    let account = Account::from_private_key(given_private_key);
    let public_key = account.to_signature_public_key();

    let signature = account.sign(given_message);

    let signature_verification = Account::verify(&public_key.to_string(), given_message, &signature);

    println!("{} == {}", true, signature_verification);
    assert!(signature_verification);
}

#[wasm_bindgen_test]
pub fn test_private_key_signature_failed_verification() {
    let given_private_key = "APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn";
    let given_message = "test message";
    let bad_message = "bad message";

    let account = Account::from_private_key(given_private_key);
    let public_key = account.to_signature_public_key();

    let signature = account.sign(given_message);

    let signature_verification = Account::verify(&public_key.to_string(), bad_message, &signature);

    println!("{} == {}", false, signature_verification);
    assert!(!signature_verification);
}

// Test Schnorr signature scheme where:
//   The Account View Key is the signature private key.
//   The Account Address is the signature public key.

#[wasm_bindgen_test]
pub fn test_view_key_signature_verification() {
    let given_view_key = "AViewKey1m8gvywHKHKfUzZiLiLoHedcdHEjKwo5TWo6efz8gK7wF";
    let given_message = "test message";

    let view_key = ViewKey::from_string(given_view_key);
    let address = Address::from_view_key(given_view_key);

    let signature = view_key.sign(given_message);

    let signature_verification = address.verify(given_message, &signature);

    println!("{} == {}", true, signature_verification);
    assert!(signature_verification);
}

#[wasm_bindgen_test]
pub fn test_view_key_signature_failed_verification() {
    let given_view_key = "AViewKey1m8gvywHKHKfUzZiLiLoHedcdHEjKwo5TWo6efz8gK7wF";
    let given_message = "test message";
    let bad_message = "bad message";

    let view_key = ViewKey::from_string(given_view_key);
    let address = Address::from_view_key(given_view_key);

    let signature = view_key.sign(given_message);

    let signature_verification = address.verify(bad_message, &signature);

    println!("{} == {}", false, signature_verification);
    assert!(!signature_verification);
}
