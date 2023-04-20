// Copyright (C) 2019-2023 Aleo Systems Inc.
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

#![forbid(unsafe_code)]

#[macro_use]
extern crate tracing;

pub mod rocksdb;

mod block;
pub use block::*;

mod consensus;
pub use consensus::*;

mod program;
pub use program::*;

mod transaction;
pub use transaction::*;

mod transition;
pub use transition::*;

/// The RocksDB map prefix broken down into the entry category and the specific type of the entry.
// Note: the order of these variants can be changed at any point in time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum MapID {
    Block(BlockMap),
    Deployment(DeploymentMap),
    Execution(ExecutionMap),
    Transaction(TransactionMap),
    Transition(TransitionMap),
    TransitionInput(TransitionInputMap),
    TransitionOutput(TransitionOutputMap),
    Program(ProgramMap),
    #[cfg(test)]
    Test(TestMap),
}

impl From<MapID> for u16 {
    fn from(id: MapID) -> u16 {
        match id {
            MapID::Block(id) => id as u16,
            MapID::Deployment(id) => id as u16,
            MapID::Execution(id) => id as u16,
            MapID::Transaction(id) => id as u16,
            MapID::Transition(id) => id as u16,
            MapID::TransitionInput(id) => id as u16,
            MapID::TransitionOutput(id) => id as u16,
            MapID::Program(id) => id as u16,
            #[cfg(test)]
            MapID::Test(id) => id as u16,
        }
    }
}

/// The RocksDB map prefix for block-related entries.
// Note: the order of these variants can be changed at any point in time,
// as long as the corresponding DataID values remain the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum BlockMap {
    StateRoot = DataID::BlockStateRootMap as u16,
    ReverseStateRoot = DataID::BlockReverseStateRootMap as u16,
    ID = DataID::BlockIDMap as u16,
    ReverseID = DataID::BlockReverseIDMap as u16,
    Header = DataID::BlockHeaderMap as u16,
    Transactions = DataID::BlockTransactionsMap as u16,
    ReverseTransactions = DataID::BlockReverseTransactionsMap as u16,
    CoinbaseSolution = DataID::BlockCoinbaseSolutionMap as u16,
    CoinbasePuzzleCommitment = DataID::BlockCoinbasePuzzleCommitmentMap as u16,
    Signature = DataID::BlockSignatureMap as u16,
}

/// The RocksDB map prefix for deployment-related entries.
// Note: the order of these variants can be changed at any point in time,
// as long as the corresponding DataID values remain the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum DeploymentMap {
    ID = DataID::DeploymentIDMap as u16,
    Edition = DataID::DeploymentEditionMap as u16,
    ReverseID = DataID::DeploymentReverseIDMap as u16,
    Owner = DataID::DeploymentOwnerMap as u16,
    Program = DataID::DeploymentProgramMap as u16,
    VerifyingKey = DataID::DeploymentVerifyingKeyMap as u16,
    Certificate = DataID::DeploymentCertificateMap as u16,
    Fee = DataID::DeploymentFeeMap as u16,
    ReverseFee = DataID::DeploymentReverseFeeMap as u16,
}

/// The RocksDB map prefix for execution-related entries.
// Note: the order of these variants can be changed at any point in time,
// as long as the corresponding DataID values remain the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ExecutionMap {
    ID = DataID::ExecutionIDMap as u16,
    ReverseID = DataID::ExecutionReverseIDMap as u16,
    Inclusion = DataID::ExecutionInclusionMap as u16,
    Fee = DataID::ExecutionFeeMap as u16,
}

/// The RocksDB map prefix for transition input entries.
// Note: the order of these variants can be changed at any point in time,
// as long as the corresponding DataID values remain the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum TransitionInputMap {
    ID = DataID::InputIDMap as u16,
    ReverseID = DataID::InputReverseIDMap as u16,
    Constant = DataID::InputConstantMap as u16,
    Public = DataID::InputPublicMap as u16,
    Private = DataID::InputPrivateMap as u16,
    Record = DataID::InputRecordMap as u16,
    RecordTag = DataID::InputRecordTagMap as u16,
    ExternalRecord = DataID::InputExternalRecordMap as u16,
}

/// The RocksDB map prefix for transition output entries.
// Note: the order of these variants can be changed at any point in time,
// as long as the corresponding DataID values remain the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum TransitionOutputMap {
    ID = DataID::OutputIDMap as u16,
    ReverseID = DataID::OutputReverseIDMap as u16,
    Constant = DataID::OutputConstantMap as u16,
    Public = DataID::OutputPublicMap as u16,
    Private = DataID::OutputPrivateMap as u16,
    Record = DataID::OutputRecordMap as u16,
    RecordNonce = DataID::OutputRecordNonceMap as u16,
    ExternalRecord = DataID::OutputExternalRecordMap as u16,
}

/// The RocksDB map prefix for transaction-related entries.
// Note: the order of these variants can be changed at any point in time,
// as long as the corresponding DataID values remain the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum TransactionMap {
    ID = DataID::TransactionIDMap as u16,
}

/// The RocksDB map prefix for transition-related entries.
// Note: the order of these variants can be changed at any point in time,
// as long as the corresponding DataID values remain the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum TransitionMap {
    Locator = DataID::TransitionLocatorMap as u16,
    Finalize = DataID::TransitionFinalizeMap as u16,
    Proof = DataID::TransitionProofMap as u16,
    TPK = DataID::TransitionTPKMap as u16,
    ReverseTPK = DataID::TransitionReverseTPKMap as u16,
    TCM = DataID::TransitionTCMMap as u16,
    ReverseTCM = DataID::TransitionReverseTCMMap as u16,
}

/// The RocksDB map prefix for program-related entries.
// Note: the order of these variants can be changed at any point in time,
// as long as the corresponding DataID values remain the same.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ProgramMap {
    ProgramID = DataID::ProgramIDMap as u16,
    ProgramIndex = DataID::ProgramIndexMap as u16,
    MappingID = DataID::MappingIDMap as u16,
    KeyValueID = DataID::KeyValueIDMap as u16,
    Key = DataID::KeyMap as u16,
    Value = DataID::ValueMap as u16,
}

/// The RocksDB map prefix for test-related entries.
// Note: the order of these variants can be changed at any point in time.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum TestMap {
    Test = DataID::Test as u16,
}

/// The RocksDB map prefix.
// Note: the order of these variants can NOT be changed once the database is populated:
// - any new variant MUST be added as the last one (ignoring the Test one)
// - any deprecated variant MUST remain in its position (it can't be removed)
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
enum DataID {
    // Block
    BlockStateRootMap,
    BlockReverseStateRootMap,
    BlockIDMap,
    BlockReverseIDMap,
    BlockHeaderMap,
    BlockTransactionsMap,
    BlockReverseTransactionsMap,
    BlockCoinbaseSolutionMap,
    BlockCoinbasePuzzleCommitmentMap,
    BlockSignatureMap,
    // Deployment
    DeploymentIDMap,
    DeploymentEditionMap,
    DeploymentReverseIDMap,
    DeploymentOwnerMap,
    DeploymentProgramMap,
    DeploymentVerifyingKeyMap,
    DeploymentCertificateMap,
    DeploymentFeeMap,
    DeploymentReverseFeeMap,
    // Execution
    ExecutionIDMap,
    ExecutionReverseIDMap,
    ExecutionInclusionMap,
    ExecutionFeeMap,
    // Input
    InputIDMap,
    InputReverseIDMap,
    InputConstantMap,
    InputPublicMap,
    InputPrivateMap,
    InputRecordMap,
    InputRecordTagMap,
    InputExternalRecordMap,
    // Output
    OutputIDMap,
    OutputReverseIDMap,
    OutputConstantMap,
    OutputPublicMap,
    OutputPrivateMap,
    OutputRecordMap,
    OutputRecordNonceMap,
    OutputExternalRecordMap,
    // Transaction
    TransactionIDMap,
    // Transition
    TransitionLocatorMap,
    TransitionFinalizeMap,
    TransitionProofMap,
    TransitionTPKMap,
    TransitionReverseTPKMap,
    TransitionTCMMap,
    TransitionReverseTCMMap,
    // Program
    ProgramIDMap,
    ProgramIndexMap,
    MappingIDMap,
    KeyValueIDMap,
    KeyMap,
    ValueMap,

    // Testing
    #[cfg(test)]
    Test,
}
