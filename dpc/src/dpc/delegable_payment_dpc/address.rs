use crate::dpc::{delegable_payment_dpc::DelegablePaymentDPCComponents, AddressKeyPair};
use snarkos_models::algorithms::{CommitmentScheme, SignatureScheme, PRF};
use snarkos_utilities::bytes::ToBytes;

use std::io::{Result as IoResult, Write};

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: DelegablePaymentDPCComponents"),
    Clone(bound = "C: DelegablePaymentDPCComponents"),
    Debug(bound = "C: DelegablePaymentDPCComponents")
)]
pub struct AddressPublicKey<C: DelegablePaymentDPCComponents> {
    pub public_key: <C::AddrC as CommitmentScheme>::Output,
}

impl<C: DelegablePaymentDPCComponents> ToBytes for AddressPublicKey<C> {
    fn write<W: Write>(&self, writer: W) -> IoResult<()> {
        self.public_key.write(writer)
    }
}

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: DelegablePaymentDPCComponents"),
    Clone(bound = "C: DelegablePaymentDPCComponents")
)]
pub struct AddressSecretKey<C: DelegablePaymentDPCComponents> {
    pub pk_sig: <C::S as SignatureScheme>::PublicKey,
    pub sk_sig: <C::S as SignatureScheme>::PrivateKey,
    pub sk_prf: <C::P as PRF>::Seed,
    pub metadata: [u8; 32],
    pub r_pk: <C::AddrC as CommitmentScheme>::Randomness,
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DelegablePaymentDPCComponents"))]
pub struct AddressPair<C: DelegablePaymentDPCComponents> {
    pub public_key: AddressPublicKey<C>,
    pub secret_key: AddressSecretKey<C>,
}

impl<C: DelegablePaymentDPCComponents> AddressKeyPair for AddressPair<C> {
    type AddressPublicKey = AddressPublicKey<C>;
    type AddressSecretKey = AddressSecretKey<C>;
}
