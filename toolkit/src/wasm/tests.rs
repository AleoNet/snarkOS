use crate::wasm::Account;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "AKEY1d6WnCerdm8CoF2qhPgQfHpfgNmmptxcZoKy6E6nvyCCK";
    let given_address = "aleo12pfv0ta2k9p6mvpsxe54zqzdc9vwqrav0trec4hzdjaj756x5vgqc2f7fl";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}
