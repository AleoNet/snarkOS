use crate::account::{Account, AccountAddress, AccountPrivateKey};
use snarkos_dpc::base_dpc::{instantiated::Components, parameters::CircuitParameters};
use snarkos_models::objects::account::AccountScheme;

use rand::thread_rng;
use std::str::FromStr;

#[test]
fn test_account_new() {
    let rng = &mut thread_rng();
    let parameters = CircuitParameters::<Components>::load().unwrap();

    let account = Account::<Components>::new(
        &parameters.account_signature,
        &parameters.account_commitment,
        &parameters.account_encryption,
        rng,
    );

    println!("{:?}", account);
    assert!(account.is_ok());

    println!("{}", account.unwrap());
}

#[test]
pub fn test_private_key_from_str() {
    let private_key_string = "AKey1zm4r3SatBhwyk681f3BXQMguhbrtUVmXDgzz4f6fDNiVhj84MDKarpNKTwpJrzEQ5FFoyAYXL3cWyrXNt3dSrw32FuK1Bg6C9ebjQax7nJ6MvCohWmaYYj3DdLWe15PopXKRi";

    let private_key = AccountPrivateKey::<Components>::from_str(private_key_string);
    println!("{:?}", private_key);

    assert!(private_key.is_ok());
    assert_eq!(private_key_string, private_key.unwrap().to_string());
}

#[test]
pub fn test_address_from_str() {
    let address_string = "aleo1z6eq6ted3p43htq3mxsacsems48rnv9tr4rvq0x37q4j3dggyvyqkt760a";

    let address = AccountAddress::<Components>::from_str(address_string);
    assert!(address.is_ok());
    assert_eq!(address_string, address.unwrap().to_string());
}
