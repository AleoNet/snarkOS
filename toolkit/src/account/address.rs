use crate::{account::PrivateKey, errors::AddressError};

use snarkos_dpc::base_dpc::{instantiated::Components, parameters::SystemParameters};
use snarkos_objects::AccountAddress;
use snarkos_utilities::bytes::ToBytes;

use std::{fmt, str::FromStr};

#[derive(Debug)]
pub struct Address {
    address: AccountAddress<Components>,
}

impl Address {
    pub fn from(private_key: &PrivateKey) -> Result<Self, AddressError> {
        let parameters = SystemParameters::<Components>::load()?;
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

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(address: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            address: AccountAddress::<Components>::from_str(address)?,
        })
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.address.to_string())
    }
}
