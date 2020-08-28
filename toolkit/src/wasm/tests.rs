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

use crate::wasm::{Account, OfflineTransaction, SignatureScheme, SignatureSchemePublicKey};

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

#[wasm_bindgen_test]
pub fn offline_transaction_test() {
    let given_private_key = "APrivateKey1tvv5YV1dipNiku2My8jMkqpqCyYKvR5Jq4y2mtjw7s77Zpn";
    let given_record = "4f6d042c3bc73e412f4b4740ad27354a1b25bb9df93f29313350356aa88dca050080d1f008000000000000000000000000000000000000000000000000000000000000000000000000304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b00304e7ae3ef9577877ddcef8f8c5d9b5e3bf544c78c50c51213857f35c33c3502df12f0fb72a0d7c56ccd31a87dada92b003f07ea7279544031efc42c1c785f4f403146e6fdbfcae26bfaa61f2d2202fd0117df47122a693ceaf27c4ceabb3c4b619333f4663bb7e85a6e741252ba1c6e11af1e1c74edf8ae1963c3532ec6e05a07f96d6731334bc368f93b428491343004";
    let given_address = "aleo1faksgtpmculyzt6tgaq26fe4fgdjtwualyljjvfn2q6k42ydegzspfz9uh";

    let offline_transaction = OfflineTransaction::create_offline_transaction(
        given_private_key,
        None,
        given_record,
        None,
        given_address,
        None,
        vec![10000],
        1,
    );

    let offline_transaction_string = offline_transaction.offline_transaction.to_string();

    // Offline transaction can be recovered
    let _offline_transaction = OfflineTransaction::from_string(&offline_transaction_string);
}
