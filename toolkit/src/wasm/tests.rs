use crate::wasm::Account;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "AKey1KqF7t1fXt4ie38R3nyqbWokKru23zkLBTiSBjyfK96matiT3FEcLjmBabc3goXPXbZKcU1XLFHBuQ75mCsjCeLo8AbWQ7DMCmCyGRJGf8RKBvXQjnL51sZgqesMMaoxujzBm6gBfx2fmAW4QbQFQqxJSojna2rd6fJYS8v5D8zmy6R1";
    let given_public_key = "aleo173xscwe62kten0v44up5e678kj7j62d24ex2htkks34cutjlssqqmwnv76";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_public_key, account.public_key.to_string());
    assert_eq!(given_public_key, account.public_key.to_string());
}
