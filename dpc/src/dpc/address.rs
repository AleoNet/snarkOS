use crate::dpc::{plain_dpc::DPCComponents, AddressKeyPair};
use snarkos_models::algorithms::{CommitmentScheme, PRF};
use snarkos_utilities::bytes::ToBytes;

use std::io::{Result as IoResult, Write};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DPCComponents"))]
pub struct AddressPair<C: DPCComponents> {
    pub public_key: AddressPublicKey<C>,
    pub secret_key: AddressSecretKey<C>,
}

impl<C: DPCComponents> AddressKeyPair for AddressPair<C> {
    type AddressPublicKey = AddressPublicKey<C>;
    type AddressSecretKey = AddressSecretKey<C>;
}

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: DPCComponents"),
    Clone(bound = "C: DPCComponents"),
    Debug(bound = "C: DPCComponents")
)]
pub struct AddressPublicKey<C: DPCComponents> {
    pub public_key: <C::AddressCommitment as CommitmentScheme>::Output,
}

impl<C: DPCComponents> ToBytes for AddressPublicKey<C> {
    fn write<W: Write>(&self, writer: W) -> IoResult<()> {
        self.public_key.write(writer)
    }
}

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: DPCComponents"),
    Clone(bound = "C: DPCComponents"),
    Debug(bound = "C: DPCComponents")
)]
pub struct AddressSecretKey<C: DPCComponents> {
    pub sk_prf: <C::P as PRF>::Seed,
    pub metadata: [u8; 32],
    pub r_pk: <C::AddressCommitment as CommitmentScheme>::Randomness,
}
