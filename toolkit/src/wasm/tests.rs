use crate::wasm::Account;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "AKey1zm4r3SatBhwyk681f3BXQMguhbrtUVmXDgzz4f6fDNiVhj84MDKarpNKTwpJrzEQ5FFoyAYXL3cWyrXNt3dSrw32FseE66etWaZBWWNTLAr32WKeNjB4Gt8nMFTuXg1JyUByJ";
    let given_address = "aleo1ps5gw9yx3lkngl9kjdgrd47fzye6jy4ws4zj37njk446sn6euvrqzc4uqk";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}
