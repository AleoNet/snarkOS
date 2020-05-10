use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    dpc::{Predicate, Record},
    objects::AccountScheme,
};
use snarkos_objects::{dpc::Transaction, ledger::*};

use rand::Rng;

pub mod base_dpc;

pub trait DPCScheme<L: Ledger> {
    type AddressKeyPair: AccountScheme;
    type Auxiliary;
    type Metadata: ?Sized;
    type Payload;
    type Parameters;
    type Predicate: Predicate<PrivateWitness = Self::PrivatePredInput>;
    type PrivatePredInput;
    type Record: Record<
        AccountPublicKey = <Self::AddressKeyPair as AccountScheme>::AccountPublicKey,
        Predicate = Self::Predicate,
    >;
    type Transaction: Transaction<SerialNumber = <Self::Record as Record>::SerialNumber>;
    type LocalData;

    /// Returns public parameters for the DPC.
    fn setup<R: Rng>(ledger_parameters: &L::Parameters, rng: &mut R) -> Result<Self::Parameters, DPCError>;

    /// Returns an address key pair, given public parameters, metadata, and an
    /// rng.
    fn create_address<R: Rng>(
        parameters: &Self::Parameters,
        metadata: &Self::Metadata,
        rng: &mut R,
    ) -> Result<Self::AddressKeyPair, DPCError>;

    /// Returns new records and a transaction based on the authorized
    /// consumption of old records.
    fn execute<R: Rng>(
        parameters: &Self::Parameters,

        old_records: &[Self::Record],
        old_account_private_keys: &[<Self::AddressKeyPair as AccountScheme>::AccountPrivateKey],
        old_private_pred_input: impl FnMut(&Self::LocalData) -> Result<Vec<Self::PrivatePredInput>, DPCError>,

        new_account_public_key: &[<Self::AddressKeyPair as AccountScheme>::AccountPublicKey],
        new_is_dummy_flags: &[bool],
        new_payloads: &[Self::Payload],
        new_birth_predicates: &[Self::Predicate],
        new_death_predicates: &[Self::Predicate],
        new_private_pred_input: impl FnMut(&Self::LocalData) -> Result<Vec<Self::PrivatePredInput>, DPCError>,

        auxiliary: &Self::Auxiliary,
        memorandum: &<Self::Transaction as Transaction>::Memorandum,
        ledger: &L,
        rng: &mut R,
    ) -> Result<(Vec<Self::Record>, Self::Transaction), DPCError>;

    /// Returns true iff the transaction is valid according to the ledger.
    fn verify(parameters: &Self::Parameters, transaction: &Self::Transaction, ledger: &L) -> Result<bool, DPCError>;

    /// Returns true iff all the transactions in the block are valid according to the ledger.
    fn verify_transactions(
        parameters: &Self::Parameters,
        block: &Vec<Self::Transaction>,
        ledger: &L,
    ) -> Result<bool, DPCError>;
}
