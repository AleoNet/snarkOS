use crate::wasm::Account;

use wasm_bindgen_test::*;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "AKEY1b47dMA8f9GfXPsW9s16qWfiYYmWGAAcorK9RkaVpBeFA";
    let given_address = "aleo1y90yg3yzs4g7q25f9nn8khuu00m8ysynxmcw8aca2d0phdx8dgpq4vw348";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}
