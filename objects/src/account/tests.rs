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
    let private_key_string = "AKEY1YXwhewzuVjBqXpALejm7TFcdqGJZDpR74B8P6Q9iwsuu";

    let private_key = AccountPrivateKey::<Components>::from_str(private_key_string);
    println!("{:?}", private_key);

    assert!(private_key.is_ok());
    assert_eq!(private_key_string, private_key.unwrap().to_string());
}

#[test]
pub fn test_address_from_str() {
    let address_string = "aleo1wztym2rpv8f7f7j47xz2tyfdsgh36u86q6ph6qkhtlfc0g0segqqlge0gm";

    let address = AccountAddress::<Components>::from_str(address_string);
    assert!(address.is_ok());
    assert_eq!(address_string, address.unwrap().to_string());
}
