use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::hash::Hash;

pub trait Record: Default + FromBytes + ToBytes {
    type AccountPublicKey;
    type Commitment: FromBytes + ToBytes;
    type CommitmentRandomness;
    type Payload;
    type Predicate;
    type SerialNumberNonce;
    type SerialNumber: Clone + Eq + Hash + FromBytes + ToBytes;
    type Value;

    /// Returns the account public key.
    fn account_public_key(&self) -> &Self::AccountPublicKey;

    /// Returns whether or not the record is dummy.
    fn is_dummy(&self) -> bool;

    /// Returns the record payload.
    fn payload(&self) -> &Self::Payload;

    /// Returns the birth predicate of this record.
    fn birth_predicate_repr(&self) -> &[u8];

    /// Returns the death predicate of this record.
    fn death_predicate_repr(&self) -> &[u8];

    /// Returns the randomness used for the serial number.
    fn serial_number_nonce(&self) -> &Self::SerialNumberNonce;

    /// Returns the commitment of this record.
    fn commitment(&self) -> Self::Commitment;

    /// Returns the randomness used for the commitment.
    fn commitment_randomness(&self) -> Self::CommitmentRandomness;

    /// Returns the record value.
    fn value(&self) -> Self::Value;
}
