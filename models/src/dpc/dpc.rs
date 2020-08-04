use crate::{
    dpc::{Program, Record},
    objects::{AccountScheme, LedgerScheme, Transaction},
};
use snarkos_errors::dpc::DPCError;

use rand::Rng;

pub trait DPCScheme<L: LedgerScheme> {
    type Account: AccountScheme;
    type Metadata: ?Sized;
    type Payload;
    type Parameters;
    type PrivateProgramInput;
    type Record: Record<Owner = <Self::Account as AccountScheme>::AccountAddress>;
    type SystemParameters;
    type Transaction: Transaction<SerialNumber = <Self::Record as Record>::SerialNumber>;
    type LocalData;
    type ExecuteContext;

    /// Returns public parameters for the DPC.
    fn setup<R: Rng>(ledger_parameters: &L::MerkleParameters, rng: &mut R) -> Result<Self::Parameters, DPCError>;

    /// Returns an account, given the public parameters, metadata, and an rng.
    fn create_account<R: Rng>(parameters: &Self::Parameters, rng: &mut R) -> Result<Self::Account, DPCError>;

    /// Returns the execution context required for program snark and DPC transaction generation.
    fn execute_offline<P: Program, R: Rng>(
        parameters: &Self::SystemParameters,
        old_records: &[Self::Record],
        old_account_private_keys: &[<Self::Account as AccountScheme>::AccountPrivateKey],
        new_record_owners: &[<Self::Account as AccountScheme>::AccountAddress],
        new_is_dummy_flags: &[bool],
        new_values: &[u64],
        new_payloads: &[Self::Payload],
        new_birth_programs: &[P],
        new_death_programs: &[P],
        memorandum: &<Self::Transaction as Transaction>::Memorandum,
        network_id: u8,
        rng: &mut R,
    ) -> Result<Self::ExecuteContext, DPCError>;

    /// Returns new records and a transaction based on the authorized
    /// consumption of old records.
    fn execute_online<R: Rng>(
        parameters: &Self::Parameters,
        execute_context: Self::ExecuteContext,
        old_death_program_proofs: &[Self::PrivateProgramInput],
        new_birth_program_proofs: &[Self::PrivateProgramInput],
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
