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

use snarkvm_errors::objects::TransactionError;
use snarkvm_utilities::bytes::{FromBytes, ToBytes};

use std::hash::Hash;

pub trait Transaction: Clone + Eq + FromBytes + ToBytes {
    type Commitment: Clone + Eq + Hash + FromBytes + ToBytes;
    type Digest: Clone + Eq + Hash + FromBytes + ToBytes;
    type InnerSNARKID: Clone + Eq + FromBytes + ToBytes;
    type LocalDataRoot: Clone + Eq + Hash + FromBytes + ToBytes;
    type Memorandum: Clone + Eq + Hash + FromBytes + ToBytes;
    type ProgramCommitment: Clone + Eq + Hash + FromBytes + ToBytes;
    type SerialNumber: Clone + Eq + Hash + FromBytes + ToBytes;
    type EncryptedRecord: Clone + Eq + FromBytes + ToBytes;
    type ValueBalance: Clone + Eq + FromBytes + ToBytes;

    /// Returns the transaction identifier.
    fn transaction_id(&self) -> Result<[u8; 32], TransactionError>;

    /// Returns the network_id in the transaction.
    fn network_id(&self) -> u8;

    /// Returns the ledger digest.
    fn ledger_digest(&self) -> &Self::Digest;

    /// Returns the inner snark id.
    fn inner_snark_id(&self) -> &Self::InnerSNARKID;

    /// Returns the old serial numbers.
    fn old_serial_numbers(&self) -> &[Self::SerialNumber];

    /// Returns the new commitments.
    fn new_commitments(&self) -> &[Self::Commitment];

    /// Returns the program commitment in the transaction.
    fn program_commitment(&self) -> &Self::ProgramCommitment;

    /// Returns the local data root in the transaction.
    fn local_data_root(&self) -> &Self::LocalDataRoot;

    /// Returns the value balance in the transaction.
    fn value_balance(&self) -> Self::ValueBalance;

    /// Returns the encrypted records
    fn encrypted_records(&self) -> &[Self::EncryptedRecord];

    /// Returns the memorandum.
    fn memorandum(&self) -> &Self::Memorandum;

    /// Returns the transaction size in bytes.
    fn size(&self) -> usize;
}
