use crate::account::{Account, AccountPrivateKey, AccountPublicKey};
use snarkos_dpc::base_dpc::{instantiated::Components, parameters::CircuitParameters};
use snarkos_models::objects::account::AccountScheme;

use rand::thread_rng;
use std::str::FromStr;

#[test]
fn test_account_new() {
    let rng = &mut thread_rng();
    let parameters = CircuitParameters::<Components>::load().unwrap();

    let account = Account::<Components>::new(&parameters.account_signature, &parameters.account_commitment, rng);
    assert!(account.is_ok());
    println!("{}", account.unwrap());
}

#[test]
pub fn test_private_key_from_str() {
    let private_key_string = "AKey1zkVoUdcBEm27iZ2i6fpz5HNs7LwZA5QGDBGJjpwj3t8PYqYgGMt7oH9zZANS2G9fMkHFNTeVpsKa8Cb5M99cTe5WhWzmwB6cd7KcXwtuXdZNG5ae8SiJJYw5jYvr8KHjS9xYp";

    let private_key = AccountPrivateKey::<Components>::from_str(private_key_string);
    println!("{:?}", private_key);

    assert!(private_key.is_ok());
    assert_eq!(private_key_string, private_key.unwrap().to_string());
}

#[test]
pub fn test_public_key_from_str() {
    let public_key_string = "aleo1r6wgqkn0dqellfu92et9aupddvflq6khwr0hgavnec2y67nwc5qskpc0vn";

    let public_key = AccountPublicKey::<Components>::from_str(public_key_string);
    assert!(public_key.is_ok());
    assert_eq!(public_key_string, public_key.unwrap().to_string());
}
