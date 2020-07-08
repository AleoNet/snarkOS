use crate::{account::PrivateKey, errors::PublicKeyError};

use snarkos_dpc::base_dpc::{instantiated::Components, parameters::CircuitParameters};
use snarkos_objects::AccountPublicKey;
use snarkos_utilities::bytes::ToBytes;

use std::fmt;

#[derive(Debug)]
pub struct PublicKey {
    public_key: AccountPublicKey<Components>,
}

impl PublicKey {
    pub fn from(private_key: &PrivateKey) -> Result<Self, PublicKeyError> {
        let parameters = CircuitParameters::<Components>::load()?;
        let public_key = AccountPublicKey::<Components>::from(
            &parameters.account_signature,
            &parameters.account_commitment,
            &private_key.private_key,
        )?;
        Ok(Self { public_key })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = vec![];
        self.public_key
            .write(&mut output)
            .expect("serialization to bytes failed");
        output
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.public_key.to_string())
    }
}
