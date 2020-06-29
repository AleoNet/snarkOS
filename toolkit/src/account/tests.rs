use crate::account::{PrivateKey, PublicKey};

use rand::SeedableRng;
use rand_chacha::ChaChaRng;

#[test]
pub fn private_key_test() {
    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
    let private_key = PrivateKey::new(None, rng);
    assert!(private_key.is_ok());

    let expected_private_key = "AKey1KqF7t1fXt4ie38R3nyqbWokKru23zkLBTiSBjyfK96matiT3FEcLjmBabc3goXPXbZKcU1XLFHBuQ75mCsjCeLo8AbWQ7DMCmCyGRJGf8RKBvXQjnL51sZgqesMMaoxujzBm6gBfx2fmAW4QbQFQqxJSojna2rd6fJYS8v5D8zmy6R1";
    let candidate_private_key = private_key.unwrap().to_string();

    println!("{} == {}", expected_private_key, candidate_private_key);
    assert_eq!(expected_private_key, candidate_private_key);
}

#[test]
pub fn public_key_test() {
    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
    let private_key = PrivateKey::new(None, rng).unwrap();
    let public_key = PublicKey::from(&private_key);
    assert!(public_key.is_ok());

    let expected_public_key = "aleo173xscwe62kten0v44up5e678kj7j62d24ex2htkks34cutjlssqqmwnv76";
    let candidate_public_key = public_key.unwrap().to_string();

    println!("{} == {}", expected_public_key, candidate_public_key);
    assert_eq!(expected_public_key, candidate_public_key);
}
