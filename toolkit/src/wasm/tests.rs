use crate::wasm::Account;

use wasm_bindgen_test::*;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "APrivateKey1yVam4m9z94xPdNw8Rxt2QSJvDGdboSsW76bxXmf6qSwG1sx";
    let given_address = "aleo1zx95xxqlx7rgn2h6j0u6fj4l9j3lwrueggrq7yw9xr99ykw94v9sl5k2tv";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}
