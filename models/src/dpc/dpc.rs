// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{
    dpc::Record,
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
    type TransactionKernel;

    /// Returns public parameters for the DPC.
    fn setup<R: Rng>(ledger_parameters: &L::MerkleParameters, rng: &mut R) -> Result<Self::Parameters, DPCError>;

    /// Returns an account, given the public parameters, metadata, and an rng.
    fn create_account<R: Rng>(parameters: &Self::Parameters, rng: &mut R) -> Result<Self::Account, DPCError>;

    /// Returns the execution context required for program snark and DPC transaction generation.
    #[allow(clippy::too_many_arguments)]
    fn execute_offline<R: Rng>(
        parameters: Self::SystemParameters,
        old_records: Vec<Self::Record>,
        old_account_private_keys: Vec<<Self::Account as AccountScheme>::AccountPrivateKey>,
        new_record_owners: Vec<<Self::Account as AccountScheme>::AccountAddress>,
        new_is_dummy_flags: &[bool],
        new_values: &[u64],
        new_payloads: Vec<Self::Payload>,
        new_birth_program_ids: Vec<Vec<u8>>,
        new_death_program_ids: Vec<Vec<u8>>,
        memorandum: <Self::Transaction as Transaction>::Memorandum,
        network_id: u8,
        rng: &mut R,
    ) -> Result<Self::TransactionKernel, DPCError>;

    /// Returns new records and a transaction based on the authorized
    /// consumption of old records.
    fn execute_online<R: Rng>(
        parameters: &Self::Parameters,
        transaction_kernel: Self::TransactionKernel,
        old_death_program_proofs: Vec<Self::PrivateProgramInput>,
        new_birth_program_proofs: Vec<Self::PrivateProgramInput>,
        ledger: &L,
        rng: &mut R,
    ) -> Result<(Vec<Self::Record>, Self::Transaction), DPCError>;

    /// Returns true iff the transaction is valid according to the ledger.
    fn verify(parameters: &Self::Parameters, transaction: &Self::Transaction, ledger: &L) -> Result<bool, DPCError>;

    /// Returns true iff all the transactions in the block are valid according to the ledger.
    fn verify_transactions(
        parameters: &Self::Parameters,
        block: &[Self::Transaction],
        ledger: &L,
    ) -> Result<bool, DPCError>;
}
