use crate::account::{Address, PrivateKey, ViewKey};

use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use std::str::FromStr;

#[test]
pub fn private_key_test() {
    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
    let private_key = PrivateKey::new(rng);
    assert!(private_key.is_ok());

    let expected_private_key = "APrivateKey1b3ixncv6hxXBqhCmybZFJVp6xJQMHC8H9WbFFGM5MAxax";
    let candidate_private_key = private_key.unwrap().to_string();

    println!("{} == {}", expected_private_key, candidate_private_key);
    assert_eq!(expected_private_key, candidate_private_key);
}

#[test]
pub fn view_key_test() {
    let private_key = PrivateKey::from_str("APrivateKey1b3ixncv6hxXBqhCmybZFJVp6xJQMHC8H9WbFFGM5MAxax").unwrap();
    let view_key = ViewKey::from(&private_key);
    assert!(view_key.is_ok());

    let expected_view_key = "AViewKey1cVP45x3E1TxJFgiJqawy7w5WQDsUXWTbpWAXSXB1sDw8";
    let candidate_view_key = view_key.unwrap().to_string();

    println!("{} == {}", expected_view_key, candidate_view_key);
    assert_eq!(expected_view_key, candidate_view_key);
}

#[test]
pub fn address_test() {
    let private_key = PrivateKey::from_str("APrivateKey1b3ixncv6hxXBqhCmybZFJVp6xJQMHC8H9WbFFGM5MAxax").unwrap();
    let address = Address::from(&private_key);
    assert!(address.is_ok());

    let expected_address = "aleo1azf0kyxw77mwz2eax7aruwe9atm2ujz7gcec2kp8wajmzn93zg8qraqpkl";
    let candidate_address = address.unwrap().to_string();

    println!("{} == {}", expected_address, candidate_address);
    assert_eq!(expected_address, candidate_address);
}
