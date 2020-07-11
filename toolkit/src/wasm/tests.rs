use crate::wasm::Account;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "AKey1zm4r3SatBhwyk681f3BXQMguhbrtUVmXDgzz4f6fDNiVhj84MDKarpNKTwpJrzEQ5FFoyAYXL3cWyrXNt3dSrw32G16XvAktSuB1Uu1MERssF9RpWqHZKHyQfdezuSnXAfPqC";
    let given_public_key = "aleo1y6gje3klg6zdxssnjfklj8zjgs9k2l7h4suydww7umf9naegnuzsy0tzyd";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_public_key, account.public_key.to_string());
    assert_eq!(given_public_key, account.public_key.to_string());
}
