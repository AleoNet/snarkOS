use crate::wasm::Account;

#[wasm_bindgen_test]
pub fn account_from_private_key_test() {
    let given_private_key = "AKey1zm4r3SatBhwyk681f3BXQMguhbrtUVmXDgzz4f6fDNiVhj84MDKarpNKTwpJrzEQ5FFoyAYXL3cWyrXNt3dSrw32FuK1Bg6C9ebjQax7nJ6MvCohWmaYYj3DdLWe15PopXKRi";
    let given_address = "aleo1z6eq6ted3p43htq3mxsacsems48rnv9tr4rvq0x37q4j3dggyvyqkt760a";

    let account = Account::from_private_key(given_private_key);

    println!("{} == {}", given_private_key, account.private_key.to_string());
    assert_eq!(given_private_key, account.private_key.to_string());

    println!("{} == {}", given_address, account.address.to_string());
    assert_eq!(given_address, account.address.to_string());
}
