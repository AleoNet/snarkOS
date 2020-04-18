use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::hash::Hash;

pub mod components;
pub use self::components::*;

pub trait AddressKeyPair {
    type AddressPublicKey: Default;
    type AddressSecretKey: Default;
}

pub trait Predicate: Clone {
    type PublicInput;
    type PrivateWitness;

    /// Returns the evaluation of the predicate on given input and witness.
    fn evaluate(&self, primary: &Self::PublicInput, witness: &Self::PrivateWitness) -> bool;

    fn into_compact_repr(&self) -> Vec<u8>;
}

pub trait Record: Default + FromBytes + ToBytes {
    type AddressPublicKey;
    type Commitment;
    type CommitmentRandomness;
    type Payload;
    type Predicate;
    type SerialNumberNonce;
    type SerialNumber: Clone + Eq + Hash + FromBytes + ToBytes;

    /// Returns the address public key.
    fn address_public_key(&self) -> &Self::AddressPublicKey;

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
}
