use crate::{account::PrivateKey, errors::PublicKeyError};

use snarkos_dpc::base_dpc::{instantiated::Components, parameters::CircuitParameters};
use snarkos_objects::AccountAddress;
use snarkos_utilities::bytes::ToBytes;

use std::fmt;

#[derive(Debug)]
pub struct Address {
    address: AccountAddress<Components>,
}

impl Address {
    pub fn from(private_key: &PrivateKey) -> Result<Self, PublicKeyError> {
        let parameters = CircuitParameters::<Components>::load()?;
        let address = AccountAddress::<Components>::from_private_key(
            &parameters.account_signature,
            &parameters.account_commitment,
            &parameters.account_encryption,
            &private_key.private_key,
        )?;
        Ok(Self { address })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = vec![];
        self.address.write(&mut output).expect("serialization to bytes failed");
        output
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.address.to_string())
    }
}
