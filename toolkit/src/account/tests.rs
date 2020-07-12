use crate::account::{Address, PrivateKey};

use rand::SeedableRng;
use rand_chacha::ChaChaRng;

#[test]
pub fn private_key_test() {
    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
    let private_key = PrivateKey::new(rng);
    assert!(private_key.is_ok());

    let expected_private_key = "AKey1zm4r3SatBhwyk681f3BXQMguhbrtUVmXDgzz4f6fDNiVhj84MDKarpNKTwpJrzEQ5FFoyAYXL3cWyrXNt3dSrw32FseE66etWaZBWWNTLAr32WKeNjB4Gt8nMFTuXg1JyUByJ";
    let candidate_private_key = private_key.unwrap().to_string();

    println!("{} == {}", expected_private_key, candidate_private_key);
    assert_eq!(expected_private_key, candidate_private_key);
}

#[test]
pub fn address_test() {
    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
    let private_key = PrivateKey::new(rng).unwrap();
    let address = Address::from(&private_key);
    assert!(address.is_ok());

    let expected_address = "aleo1ps5gw9yx3lkngl9kjdgrd47fzye6jy4ws4zj37njk446sn6euvrqzc4uqk";
    let candidate_address = address.unwrap().to_string();

    println!("{} == {}", expected_address, candidate_address);
    assert_eq!(expected_address, candidate_address);
}
