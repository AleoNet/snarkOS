use crate::{AccountPrivateKey, AccountPublicKey};
use snarkos_models::dpc::{AccountScheme, DPCComponents};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::io::{Read, Result as IoResult, Write};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DPCComponents"))]
pub struct Account<C: DPCComponents> {
    pub public_key: AccountPublicKey<C>,
    pub private_key: AccountPrivateKey<C>,
}

impl<C: DPCComponents> AccountScheme for Account<C> {
    type AccountPrivateKey = AccountPrivateKey<C>;
    type AccountPublicKey = AccountPublicKey<C>;
}

impl<C: DPCComponents> ToBytes for Account<C> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.public_key.write(&mut writer)?;
        self.private_key.write(&mut writer)
    }
}

impl<C: DPCComponents> FromBytes for Account<C> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let public_key: AccountPublicKey<C> = FromBytes::read(&mut reader)?;
        let private_key: AccountPrivateKey<C> = FromBytes::read(&mut reader)?;

        Ok(Self {
            public_key,
            private_key,
        })
    }
}
