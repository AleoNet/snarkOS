// Copyright (C) 2019-2022 Aleo Systems Inc.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum DataID {
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
    DeploymentProgramMap,
    DeploymentVerifyingKeyMap,
    DeploymentCertificateMap,
    DeploymentFeeMap,
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
    TransitionFeeMap,
    // Program
    ProgramIDMap,
    MappingIDMap,
    KeyValueIDMap,
    KeyMap,
    ValueMap,
    // Testing
    #[cfg(test)]
    Test,
}
