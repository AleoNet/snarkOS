use crate::dpc::AddressKeyPair;
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, PRF},
    dpc::DPCComponents,
};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::io::{Read, Result as IoResult, Write};

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

impl<C: DPCComponents> FromBytes for AddressPublicKey<C> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let public_key: <C::AddressCommitment as CommitmentScheme>::Output = FromBytes::read(&mut reader)?;

        Ok(Self { public_key })
    }
}

#[derive(Derivative)]
#[derivative(Default(bound = "C: DPCComponents"), Clone(bound = "C: DPCComponents"))]
pub struct AddressSecretKey<C: DPCComponents> {
    pub pk_sig: <C::Signature as SignatureScheme>::PublicKey,
    pub sk_sig: <C::Signature as SignatureScheme>::PrivateKey,
    pub sk_prf: <C::PRF as PRF>::Seed,
    pub metadata: [u8; 32],
    pub r_pk: <C::AddressCommitment as CommitmentScheme>::Randomness,
}
