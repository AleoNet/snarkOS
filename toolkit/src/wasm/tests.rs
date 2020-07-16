use crate::wasm::Account;

use wasm_bindgen_test::*;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "AKEY1jX43RQfASsUBYGfMK7AtD3Dr1uymW7AvJJaxdvjC39BZ";
    let given_address = "aleo1w9m3uj7ehcas6j7dw7jayjed9m3ppz40ftjwwawcfm3zfet8ny8q2c3sak";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}
