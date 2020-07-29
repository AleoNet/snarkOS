use crate::wasm::Account;

use wasm_bindgen_test::*;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "AKEY1kcAwj8V7VZwQAUKsdDmCxvfyTSXbZXDWwSJrSH818Jyt";
    let given_address = "aleo18q9acgakkg2fhyv66rjs77n553cl9nwkyu5h8f3fqxsyty98gc9s2s9tc0";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}
