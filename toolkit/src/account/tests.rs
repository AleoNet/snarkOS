use crate::account::{Address, PrivateKey};

use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use std::str::FromStr;

#[test]
pub fn private_key_test() {
    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
    let private_key = PrivateKey::new(rng);
    assert!(private_key.is_ok());

    let expected_private_key = "AKEY1kcAwj8V7VZwQAUKsdDmCxvfyTSXbZXDWwSJrSH818Jyt";
    let candidate_private_key = private_key.unwrap().to_string();

    println!("{} == {}", expected_private_key, candidate_private_key);
    assert_eq!(expected_private_key, candidate_private_key);
}

#[test]
pub fn address_test() {
    let private_key = PrivateKey::from_str("AKEY1kcAwj8V7VZwQAUKsdDmCxvfyTSXbZXDWwSJrSH818Jyt").unwrap();
    let address = Address::from(&private_key);
    assert!(address.is_ok());

    let expected_address = "aleo18q9acgakkg2fhyv66rjs77n553cl9nwkyu5h8f3fqxsyty98gc9s2s9tc0";
    let candidate_address = address.unwrap().to_string();

    println!("{} == {}", expected_address, candidate_address);
    assert_eq!(expected_address, candidate_address);
}
