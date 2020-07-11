use crate::account::{PrivateKey, PublicKey};

use rand::SeedableRng;
use rand_chacha::ChaChaRng;

#[test]
pub fn private_key_test() {
    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
    let private_key = PrivateKey::new(rng);
    assert!(private_key.is_ok());

    let expected_private_key = "AKey1zm4r3SatBhwyk681f3BXQMguhbrtUVmXDgzz4f6fDNiVhj84MDKarpNKTwpJrzEQ5FFoyAYXL3cWyrXNt3dSrw32G16XvAktSuB1Uu1MERssF9RpWqHZKHyQfdezuSnXAfPqC";
    let candidate_private_key = private_key.unwrap().to_string();

    println!("{} == {}", expected_private_key, candidate_private_key);
    assert_eq!(expected_private_key, candidate_private_key);
}

#[test]
pub fn public_key_test() {
    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);
    let private_key = PrivateKey::new(rng).unwrap();
    let public_key = PublicKey::from(&private_key);
    assert!(public_key.is_ok());

    let expected_public_key = "aleo1y6gje3klg6zdxssnjfklj8zjgs9k2l7h4suydww7umf9naegnuzsy0tzyd";
    let candidate_public_key = public_key.unwrap().to_string();

    println!("{} == {}", expected_public_key, candidate_public_key);
    assert_eq!(expected_public_key, candidate_public_key);
}
