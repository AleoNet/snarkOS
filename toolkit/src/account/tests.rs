use crate::account::{Address, PrivateKey, ViewKey};

use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use std::str::FromStr;

#[test]
pub fn private_key_test() {
    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
    let private_key = PrivateKey::new(rng);
    assert!(private_key.is_ok());

    let expected_private_key = "APrivateKey1yVam4m9z94xPdNw8Rxt2QSJvDGdboSsW76bxXmf6qSwG1sx";
    let candidate_private_key = private_key.unwrap().to_string();

    println!("{} == {}", expected_private_key, candidate_private_key);
    assert_eq!(expected_private_key, candidate_private_key);
}

#[test]
pub fn view_key_test() {
    let private_key = PrivateKey::from_str("APrivateKey1yVam4m9z94xPdNw8Rxt2QSJvDGdboSsW76bxXmf6qSwG1sx").unwrap();
    let view_key = ViewKey::from(&private_key);
    assert!(view_key.is_ok());

    let expected_view_key = "AViewKey1gpXLhxkMQUNbakFGAWyiM7TEfua5LvqTaTKJ1MhJ5RgU";
    let candidate_view_key = view_key.unwrap().to_string();

    println!("{} == {}", expected_view_key, candidate_view_key);
    assert_eq!(expected_view_key, candidate_view_key);
}

#[test]
pub fn address_test() {
    let private_key = PrivateKey::from_str("APrivateKey1yVam4m9z94xPdNw8Rxt2QSJvDGdboSsW76bxXmf6qSwG1sx").unwrap();
    let address = Address::from(&private_key);
    assert!(address.is_ok());

    let expected_address = "aleo1zx95xxqlx7rgn2h6j0u6fj4l9j3lwrueggrq7yw9xr99ykw94v9sl5k2tv";
    let candidate_address = address.unwrap().to_string();

    println!("{} == {}", expected_address, candidate_address);
    assert_eq!(expected_address, candidate_address);
}
