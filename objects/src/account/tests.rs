use crate::account::{Account, AccountAddress, AccountPrivateKey, AccountViewKey};
use snarkos_dpc::base_dpc::{instantiated::Components, parameters::SystemParameters};
use snarkos_models::objects::account::AccountScheme;

use rand::thread_rng;
use std::str::FromStr;

#[test]
fn test_account_new() {
    let rng = &mut thread_rng();
    let parameters = SystemParameters::<Components>::load().unwrap();

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
    let private_key_string = "APrivateKey1b5U31G1VUt6G9mAh6tm364eWgLUabK1qTBdRJKUEFEwcz";
    let private_key = AccountPrivateKey::<Components>::from_str(private_key_string);
    println!("{:?}", private_key);

    assert!(private_key.is_ok());
    assert_eq!(private_key_string, private_key.unwrap().to_string());
}

#[test]
pub fn test_view_key_from_str() {
    let view_key_string = "AViewKey1m8TjBbmN9rmorg8GEyqv4pBZ7WMnDBnTj8v8KQdUYwf6";
    let view_key = AccountViewKey::<Components>::from_str(view_key_string);
    println!("{:?}", view_key);

    assert!(view_key.is_ok());
    assert_eq!(view_key_string, view_key.unwrap().to_string());
}

#[test]
pub fn test_address_from_str() {
    let address_string = "aleo1fuge6ah8c9custvmlju5t30gk8p8lar5x36jlfa2glhgy9n0fuxsreeh2c";
    let address = AccountAddress::<Components>::from_str(address_string);
    assert!(address.is_ok());
    assert_eq!(address_string, address.unwrap().to_string());
}
