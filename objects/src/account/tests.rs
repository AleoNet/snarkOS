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
        &[0u8; 32],
        rng,
    );
    assert!(account.is_ok());
    println!("{}", account.unwrap());
}

#[test]
pub fn test_private_key_from_str() {
    let private_key_string = "AKey1PJBq5DcamxmsfM8pRL6Vowi6tSPozrqbpZzBvk9nbGmM1gMJACtbCA2SQku1DFDnG8f4Lw3jYNEoTp2XQ78MUy6bjxTw2SuVqzACs3rGquduSwJE82h7UM6UvRufrrcBByiR91USfuPEK6aUvcz6D34kqhqRPLJktVkkDEqJDYc7AyE";

    let private_key = AccountPrivateKey::<Components>::from_str(private_key_string);
    println!("{:?}", private_key);

    assert!(private_key.is_ok());
    assert_eq!(private_key_string, private_key.unwrap().to_string());
}

#[test]
pub fn test_public_key_from_str() {
    let public_key_string = "aleo1qnr4dkkvkgfqph0vzc3y6z2eu975wnpz2925ntjccd5cfqxtyu8sta57j8";

    let public_key = AccountPublicKey::<Components>::from_str(public_key_string);
    assert!(public_key.is_ok());
    assert_eq!(public_key_string, public_key.unwrap().to_string());
}
