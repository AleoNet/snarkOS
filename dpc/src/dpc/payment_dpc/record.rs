use crate::dpc::{
    payment_dpc::{
        address::AddressPublicKey,
        predicate::DPCPredicate,
        record_payload::PaymentRecordPayload,
        PaymentDPCComponents,
    },
    Record,
};
use snarkos_models::algorithms::{CommitmentScheme, CRH, PRF};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use std::marker::PhantomData;

#[derive(Derivative)]
#[derivative(Default(bound = "C: PaymentDPCComponents"), Clone(bound = "C: PaymentDPCComponents"))]
pub struct DPCRecord<C: PaymentDPCComponents> {
    pub(super) address_public_key: AddressPublicKey<C>,

    pub(super) is_dummy: bool,
    pub(super) payload: PaymentRecordPayload,

    #[derivative(Default(value = "default_predicate_hash::<C::PredVkH>()"))]
    pub(super) birth_predicate_repr: Vec<u8>,
    #[derivative(Default(value = "default_predicate_hash::<C::PredVkH>()"))]
    pub(super) death_predicate_repr: Vec<u8>,

    pub(super) serial_number_nonce: <C::SnNonceH as CRH>::Output,

    pub(super) commitment: <C::RecC as CommitmentScheme>::Output,
    pub(super) commitment_randomness: <C::RecC as CommitmentScheme>::Randomness,

    pub(super) _components: PhantomData<C>,
}

fn default_predicate_hash<C: CRH>() -> Vec<u8> {
    to_bytes![C::Output::default()].unwrap()
}

impl<C: PaymentDPCComponents> Record for DPCRecord<C> {
    type AddressPublicKey = AddressPublicKey<C>;
    type Commitment = <C::RecC as CommitmentScheme>::Output;
    type CommitmentRandomness = <C::RecC as CommitmentScheme>::Randomness;
    type Payload = PaymentRecordPayload;
    type Predicate = DPCPredicate<C>;
    type SerialNumber = <C::P as PRF>::Output;
    type SerialNumberNonce = <C::SnNonceH as CRH>::Output;

    fn address_public_key(&self) -> &Self::AddressPublicKey {
        &self.address_public_key
    }

    fn is_dummy(&self) -> bool {
        self.is_dummy
    }

    fn payload(&self) -> &Self::Payload {
        &self.payload
    }

    fn birth_predicate_repr(&self) -> &[u8] {
        &self.birth_predicate_repr
    }

    fn death_predicate_repr(&self) -> &[u8] {
        &self.death_predicate_repr
    }

    fn serial_number_nonce(&self) -> &Self::SerialNumberNonce {
        &self.serial_number_nonce
    }

    fn commitment(&self) -> Self::Commitment {
        self.commitment.clone()
    }

    fn commitment_randomness(&self) -> Self::CommitmentRandomness {
        self.commitment_randomness.clone()
    }
}
