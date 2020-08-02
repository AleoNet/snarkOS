use crate::wasm::Account;

use wasm_bindgen_test::*;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "APrivateKey1b5U31G1VUt6G9mAh6tm364eWgLUabK1qTBdRJKUEFEwcz";
    let given_address = "aleo1fuge6ah8c9custvmlju5t30gk8p8lar5x36jlfa2glhgy9n0fuxsreeh2c";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}
