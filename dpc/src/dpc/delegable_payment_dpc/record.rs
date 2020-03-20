use crate::dpc::{
    delegable_payment_dpc::{
        address::AddressPublicKey,
        predicate::DPCPredicate,
        record_payload::PaymentRecordPayload,
        DelegablePaymentDPCComponents,
    },
    Record,
};
use snarkos_models::algorithms::{CommitmentScheme, SignatureScheme, CRH};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use std::marker::PhantomData;

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: DelegablePaymentDPCComponents"),
    Clone(bound = "C: DelegablePaymentDPCComponents")
)]
pub struct DPCRecord<C: DelegablePaymentDPCComponents> {
    pub(super) address_public_key: AddressPublicKey<C>,

    pub(super) is_dummy: bool,
    pub(super) payload: PaymentRecordPayload,

    #[derivative(Default(value = "default_predicate_hash::<C::PredicateVerificationKeyHash>()"))]
    pub(super) birth_predicate_repr: Vec<u8>,
    #[derivative(Default(value = "default_predicate_hash::<C::PredicateVerificationKeyHash>()"))]
    pub(super) death_predicate_repr: Vec<u8>,

    pub(super) serial_number_nonce: <C::SerialNumberNonce as CRH>::Output,

    pub(super) commitment: <C::RecordCommitment as CommitmentScheme>::Output,
    pub(super) commitment_randomness: <C::RecordCommitment as CommitmentScheme>::Randomness,

    pub(super) _components: PhantomData<C>,
}

fn default_predicate_hash<C: CRH>() -> Vec<u8> {
    to_bytes![C::Output::default()].unwrap()
}

impl<C: DelegablePaymentDPCComponents> Record for DPCRecord<C> {
    type AddressPublicKey = AddressPublicKey<C>;
    type Commitment = <C::RecordCommitment as CommitmentScheme>::Output;
    type CommitmentRandomness = <C::RecordCommitment as CommitmentScheme>::Randomness;
    type Payload = PaymentRecordPayload;
    type Predicate = DPCPredicate<C>;
    type SerialNumber = <C::Signature as SignatureScheme>::PublicKey;
    type SerialNumberNonce = <C::SerialNumberNonce as CRH>::Output;

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
