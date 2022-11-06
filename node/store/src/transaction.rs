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

use crate::{
    rocksdb::{self, DataMap, Database},
    DataID,
    TransitionDB,
};
use snarkvm::prelude::*;

/// A database transaction storage.
#[derive(Clone)]
pub struct TransactionDB<N: Network> {
    /// The mapping of `transaction ID` to `transaction type`.
    id_map: DataMap<N::TransactionID, TransactionType>,
    /// The deployment store.
    deployment_store: DeploymentStore<N, DeploymentDB<N>>,
    /// The execution store.
    execution_store: ExecutionStore<N, ExecutionDB<N>>,
}

#[rustfmt::skip]
impl<N: Network> TransactionStorage<N> for TransactionDB<N> {
    type IDMap = DataMap<N::TransactionID, TransactionType>;
    type DeploymentStorage = DeploymentDB<N>;
    type ExecutionStorage = ExecutionDB<N>;
    type TransitionStorage = TransitionDB<N>;

    /// Initializes the transaction storage.
    fn open(transition_store: TransitionStore<N, Self::TransitionStorage>) -> Result<Self> {
        // Initialize the deployment store.
        let deployment_store = DeploymentStore::<N, DeploymentDB<N>>::open(transition_store.clone())?;
        // Initialize the execution store.
        let execution_store = ExecutionStore::<N, ExecutionDB<N>>::open(transition_store)?;
        // Return the transaction storage.
        Ok(Self { id_map: rocksdb::RocksDB::open_map(N::ID, execution_store.dev(), DataID::TransactionIDMap)?, deployment_store, execution_store })
    }

    /// Returns the ID map.
    fn id_map(&self) -> &Self::IDMap {
        &self.id_map
    }

    /// Returns the deployment store.
    fn deployment_store(&self) -> &DeploymentStore<N, Self::DeploymentStorage> {
        &self.deployment_store
    }

    /// Returns the execution store.
    fn execution_store(&self) -> &ExecutionStore<N, Self::ExecutionStorage> {
        &self.execution_store
    }
}

/// A database deployment storage.
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct DeploymentDB<N: Network> {
    /// The ID map.
    id_map: DataMap<N::TransactionID, ProgramID<N>>,
    /// The edition map.
    edition_map: DataMap<ProgramID<N>, u16>,
    /// The reverse ID map.
    reverse_id_map: DataMap<(ProgramID<N>, u16), N::TransactionID>,
    /// The program map.
    program_map: DataMap<(ProgramID<N>, u16), Program<N>>,
    /// The verifying key map.
    verifying_key_map: DataMap<(ProgramID<N>, Identifier<N>, u16), VerifyingKey<N>>,
    /// The certificate map.
    certificate_map: DataMap<(ProgramID<N>, Identifier<N>, u16), Certificate<N>>,
    /// The fee map.
    fee_map: DataMap<N::TransactionID, (N::TransitionID, N::StateRoot, Option<Proof<N>>)>,
    /// The transition store.
    transition_store: TransitionStore<N, TransitionDB<N>>,
}

#[rustfmt::skip]
impl<N: Network> DeploymentStorage<N> for DeploymentDB<N> {
    type IDMap = DataMap<N::TransactionID, ProgramID<N>>;
    type EditionMap = DataMap<ProgramID<N>, u16>;
    type ReverseIDMap = DataMap<(ProgramID<N>, u16), N::TransactionID>;
    type ProgramMap = DataMap<(ProgramID<N>, u16), Program<N>>;
    type VerifyingKeyMap = DataMap<(ProgramID<N>, Identifier<N>, u16), VerifyingKey<N>>;
    type CertificateMap = DataMap<(ProgramID<N>, Identifier<N>, u16), Certificate<N>>;
    type FeeMap = DataMap<N::TransactionID, (N::TransitionID, N::StateRoot, Option<Proof<N>>)>;
    type TransitionStorage = TransitionDB<N>;

    /// Initializes the deployment storage.
    fn open(transition_store: TransitionStore<N, Self::TransitionStorage>) -> Result<Self> {
        // Retrieve the optional development ID.
        let dev = transition_store.dev();
        Ok(Self {
            id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::DeploymentIDMap)?,
            edition_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::DeploymentEditionMap)?,
            reverse_id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::DeploymentReverseIDMap)?,
            program_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::DeploymentProgramMap)?,
            verifying_key_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::DeploymentVerifyingKeyMap)?,
            certificate_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::DeploymentCertificateMap)?,
            fee_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::DeploymentFeeMap)?,
            transition_store,
        })
    }

    /// Returns the ID map.
    fn id_map(&self) -> &Self::IDMap {
        &self.id_map
    }

    /// Returns the edition map.
    fn edition_map(&self) -> &Self::EditionMap {
        &self.edition_map
    }

    /// Returns the reverse ID map.
    fn reverse_id_map(&self) -> &Self::ReverseIDMap {
        &self.reverse_id_map
    }

    /// Returns the program map.
    fn program_map(&self) -> &Self::ProgramMap {
        &self.program_map
    }

    /// Returns the verifying key map.
    fn verifying_key_map(&self) -> &Self::VerifyingKeyMap {
        &self.verifying_key_map
    }

    /// Returns the certificate map.
    fn certificate_map(&self) -> &Self::CertificateMap {
        &self.certificate_map
    }

    /// Returns the fee map.
    fn fee_map(&self) -> &Self::FeeMap {
        &self.fee_map
    }

    /// Returns the transition store.
    fn transition_store(&self) -> &TransitionStore<N, Self::TransitionStorage> {
        &self.transition_store
    }
}

/// A database execution storage.
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct ExecutionDB<N: Network> {
    /// The ID map.
    id_map: DataMap<N::TransactionID, (Vec<N::TransitionID>, Option<N::TransitionID>)>,
    /// The reverse ID map.
    reverse_id_map: DataMap<N::TransitionID, N::TransactionID>,
    /// The transition store.
    transition_store: TransitionStore<N, TransitionDB<N>>,
    /// The inclusion map.
    inclusion_map: DataMap<N::TransactionID, (N::StateRoot, Option<Proof<N>>)>,
    /// The fee map.
    fee_map: DataMap<N::TransactionID, (N::StateRoot, Option<Proof<N>>)>,
}

#[rustfmt::skip]
impl<N: Network> ExecutionStorage<N> for ExecutionDB<N> {
    type IDMap = DataMap<N::TransactionID, (Vec<N::TransitionID>, Option<N::TransitionID>)>;
    type ReverseIDMap = DataMap<N::TransitionID, N::TransactionID>;
    type TransitionStorage = TransitionDB<N>;
    type InclusionMap = DataMap<N::TransactionID, (N::StateRoot, Option<Proof<N>>)>;
    type FeeMap = DataMap<N::TransactionID, (N::StateRoot, Option<Proof<N>>)>;

    /// Initializes the execution storage.
    fn open(transition_store: TransitionStore<N, Self::TransitionStorage>) -> Result<Self> {
        // Retrieve the optional development ID.
        let dev = transition_store.dev();
        Ok(Self {
            id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::ExecutionIDMap)?,
            reverse_id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::ExecutionReverseIDMap)?,
            transition_store,
            inclusion_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::ExecutionInclusionMap)?,
            fee_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::ExecutionFeeMap)?,
        })
    }

    /// Returns the ID map.
    fn id_map(&self) -> &Self::IDMap {
        &self.id_map
    }

    /// Returns the reverse ID map.
    fn reverse_id_map(&self) -> &Self::ReverseIDMap {
        &self.reverse_id_map
    }

    /// Returns the transition store.
    fn transition_store(&self) -> &TransitionStore<N, Self::TransitionStorage> {
        &self.transition_store
    }

    /// Returns the inclusion map.
    fn inclusion_map(&self) -> &Self::InclusionMap {
        &self.inclusion_map
    }

    /// Returns the fee map.
    fn fee_map(&self) -> &Self::FeeMap {
        &self.fee_map
    }
}
