use crate::dpc::{plain_dpc::PlainDPCComponents, AddressKeyPair};
use snarkos_models::algorithms::{CommitmentScheme, PRF};
use snarkos_utilities::bytes::ToBytes;

use std::io::{Result as IoResult, Write};

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: PlainDPCComponents"),
    Clone(bound = "C: PlainDPCComponents"),
    Debug(bound = "C: PlainDPCComponents")
)]
pub struct AddressPublicKey<C: PlainDPCComponents> {
    pub public_key: <C::AddrC as CommitmentScheme>::Output,
}

impl<C: PlainDPCComponents> ToBytes for AddressPublicKey<C> {
    fn write<W: Write>(&self, writer: W) -> IoResult<()> {
        self.public_key.write(writer)
    }
}

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: PlainDPCComponents"),
    Clone(bound = "C: PlainDPCComponents"),
    Debug(bound = "C: PlainDPCComponents")
)]
pub struct AddressSecretKey<C: PlainDPCComponents> {
    pub sk_prf:   <C::P as PRF>::Seed,
    pub metadata: [u8; 32],
    pub r_pk:     <C::AddrC as CommitmentScheme>::Randomness,
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: PlainDPCComponents"))]
pub struct AddressPair<C: PlainDPCComponents> {
    pub public_key: AddressPublicKey<C>,
    pub secret_key: AddressSecretKey<C>,
}

impl<C: PlainDPCComponents> AddressKeyPair for AddressPair<C> {
    type AddressSecretKey = AddressSecretKey<C>;
    type AddressPublicKey = AddressPublicKey<C>;
}
