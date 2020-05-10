use crate::dpc::base_dpc::{predicate::DPCPredicate, record_payload::PaymentRecordPayload, BaseDPCComponents};
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, CRH},
    dpc::Record,
};
use snarkos_objects::AccountPublicKey;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
    variable_length_integer::*,
};

use std::{
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
};

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: BaseDPCComponents"),
    Debug(bound = "C: BaseDPCComponents"),
    Clone(bound = "C: BaseDPCComponents")
)]
pub struct DPCRecord<C: BaseDPCComponents> {
    pub(super) account_public_key: AccountPublicKey<C>,

    pub(super) is_dummy: bool,
    pub(super) payload: PaymentRecordPayload,

    #[derivative(Default(value = "default_predicate_hash::<C::PredicateVerificationKeyHash>()"))]
    pub(super) birth_predicate_repr: Vec<u8>,
    #[derivative(Default(value = "default_predicate_hash::<C::PredicateVerificationKeyHash>()"))]
    pub(super) death_predicate_repr: Vec<u8>,

    pub(super) serial_number_nonce: <C::SerialNumberNonceCRH as CRH>::Output,

    pub(super) commitment: <C::RecordCommitment as CommitmentScheme>::Output,
    pub(super) commitment_randomness: <C::RecordCommitment as CommitmentScheme>::Randomness,

    pub(super) _components: PhantomData<C>,
}

fn default_predicate_hash<C: CRH>() -> Vec<u8> {
    to_bytes![C::Output::default()].unwrap()
}

impl<C: BaseDPCComponents> Record for DPCRecord<C> {
    type AccountPublicKey = AccountPublicKey<C>;
    type Commitment = <C::RecordCommitment as CommitmentScheme>::Output;
    type CommitmentRandomness = <C::RecordCommitment as CommitmentScheme>::Randomness;
    type Payload = PaymentRecordPayload;
    type Predicate = DPCPredicate<C>;
    type SerialNumber = <C::Signature as SignatureScheme>::PublicKey;
    type SerialNumberNonce = <C::SerialNumberNonceCRH as CRH>::Output;

    fn account_public_key(&self) -> &Self::AccountPublicKey {
        &self.account_public_key
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

impl<C: BaseDPCComponents> ToBytes for DPCRecord<C> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.account_public_key.write(&mut writer)?;

        self.is_dummy.write(&mut writer)?;
        self.payload.write(&mut writer)?;

        variable_length_integer(self.birth_predicate_repr.len() as u64).write(&mut writer)?;
        self.birth_predicate_repr.write(&mut writer)?;

        variable_length_integer(self.death_predicate_repr.len() as u64).write(&mut writer)?;
        self.death_predicate_repr.write(&mut writer)?;

        self.serial_number_nonce.write(&mut writer)?;
        self.commitment.write(&mut writer)?;
        self.commitment_randomness.write(&mut writer)
    }
}

impl<C: BaseDPCComponents> FromBytes for DPCRecord<C> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let account_public_key: AccountPublicKey<C> = FromBytes::read(&mut reader)?;
        let is_dummy: bool = FromBytes::read(&mut reader)?;
        let payload: PaymentRecordPayload = FromBytes::read(&mut reader)?;

        let birth_pred_repr_size: usize = read_variable_length_integer(&mut reader)?;

        let mut birth_pred_repr = vec![];
        for _ in 0..birth_pred_repr_size {
            let byte: u8 = FromBytes::read(&mut reader)?;
            birth_pred_repr.push(byte);
        }

        let death_pred_repr_size: usize = read_variable_length_integer(&mut reader)?;

        let mut death_pred_repr = vec![];
        for _ in 0..death_pred_repr_size {
            let byte: u8 = FromBytes::read(&mut reader)?;
            death_pred_repr.push(byte);
        }

        let serial_number_nonce: <C::SerialNumberNonceCRH as CRH>::Output = FromBytes::read(&mut reader)?;

        let commitment: <C::RecordCommitment as CommitmentScheme>::Output = FromBytes::read(&mut reader)?;
        let commitment_randomness: <C::RecordCommitment as CommitmentScheme>::Randomness =
            FromBytes::read(&mut reader)?;

        Ok(Self {
            account_public_key,
            is_dummy,
            payload,
            birth_predicate_repr: birth_pred_repr.to_vec(),
            death_predicate_repr: death_pred_repr.to_vec(),
            serial_number_nonce,
            commitment,
            commitment_randomness,
            _components: PhantomData,
        })
    }
}
