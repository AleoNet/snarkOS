use crate::account::{Account, AccountPrivateKey, AccountPublicKey};
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
    let private_key_string = "AKey1zm4r3SatBhwyk681f3BXQMguhbrtUVmXDgzz4f6fDNiVhj84MDKarpNKTwpJrzEQ5FFoyAYXL3cWyrXNt3dSrw32Fut2bv8e9uowfSbmKrVjPv1uF1xKn4f8V5HmpYEdUxS9g";

    let private_key = AccountPrivateKey::<Components>::from_str(private_key_string);
    println!("{:?}", private_key);

    assert!(private_key.is_ok());
    assert_eq!(private_key_string, private_key.unwrap().to_string());
}

#[test]
pub fn test_public_key_from_str() {
    let public_key_string = "aleo1xuelf4cm7amwe44p822y9qgc3m2gn4trjsn8lksjqeznq3462szql8jz4y";

    let public_key = AccountPublicKey::<Components>::from_str(public_key_string);
    assert!(public_key.is_ok());
    assert_eq!(public_key_string, public_key.unwrap().to_string());
}
