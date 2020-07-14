use crate::wasm::Account;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "AKEY1YXwhewzuVjBqXpALejm7TFcdqGJZDpR74B8P6Q9iwsuu";
    let given_address = "aleo1wztym2rpv8f7f7j47xz2tyfdsgh36u86q6ph6qkhtlfc0g0segqqlge0gm";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}
