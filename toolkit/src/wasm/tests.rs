use crate::wasm::Account;

use wasm_bindgen_test::*;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "AKEY1hES1RbMfbcybzaiwFYm7JnY1D1xNqaYF5vWPK5ejf2Nm";
    let given_address = "aleo1p3nt2dk5w4hf007ruc88nxa5amnhufrm6lcet255a93ktw9905yqqeu4rg";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}
