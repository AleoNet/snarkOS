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

use crate::wasm::{Account, SignatureScheme, SignatureSchemePublicKey, ViewKey};

use wasm_bindgen_test::*;

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
