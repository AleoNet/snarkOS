use crate::wasm::Account;

use wasm_bindgen_test::*;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "APrivateKey1b3ixncv6hxXBqhCmybZFJVp6xJQMHC8H9WbFFGM5MAxax";
    let given_address = "aleo1azf0kyxw77mwz2eax7aruwe9atm2ujz7gcec2kp8wajmzn93zg8qraqpkl";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}
