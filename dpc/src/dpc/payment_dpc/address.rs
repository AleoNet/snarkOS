use crate::dpc::{payment_dpc::PaymentDPCComponents, AddressKeyPair};
use snarkos_models::algorithms::{CommitmentScheme, PRF};
use snarkos_utilities::bytes::ToBytes;

use std::io::{Result as IoResult, Write};

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: PaymentDPCComponents"),
    Clone(bound = "C: PaymentDPCComponents"),
    Debug(bound = "C: PaymentDPCComponents")
)]
pub struct AddressPublicKey<C: PaymentDPCComponents> {
    pub public_key: <C::AddrC as CommitmentScheme>::Output,
}

impl<C: PaymentDPCComponents> ToBytes for AddressPublicKey<C> {
    fn write<W: Write>(&self, writer: W) -> IoResult<()> {
        self.public_key.write(writer)
    }
}

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: PaymentDPCComponents"),
    Clone(bound = "C: PaymentDPCComponents"),
    Debug(bound = "C: PaymentDPCComponents")
)]
pub struct AddressSecretKey<C: PaymentDPCComponents> {
    pub sk_prf: <C::P as PRF>::Seed,
    pub metadata: [u8; 32],
    pub r_pk: <C::AddrC as CommitmentScheme>::Randomness,
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: PaymentDPCComponents"))]
pub struct AddressPair<C: PaymentDPCComponents> {
    pub public_key: AddressPublicKey<C>,
    pub secret_key: AddressSecretKey<C>,
}

impl<C: PaymentDPCComponents> AddressKeyPair for AddressPair<C> {
    type AddressPublicKey = AddressPublicKey<C>;
    type AddressSecretKey = AddressSecretKey<C>;
}
