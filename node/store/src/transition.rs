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
};
use snarkvm::prelude::*;

/// A database transition storage.
#[derive(Clone)]
pub struct TransitionDB<N: Network> {
    /// The transition program IDs and function names.
    locator_map: DataMap<N::TransitionID, (ProgramID<N>, Identifier<N>)>,
    /// The transition input store.
    input_store: InputStore<N, InputDB<N>>,
    /// The transition output store.
    output_store: OutputStore<N, OutputDB<N>>,
    /// The transition proofs.
    proof_map: DataMap<N::TransitionID, Proof<N>>,
    /// The transition finalize inputs.
    finalize_map: DataMap<N::TransitionID, Option<Vec<Value<N>>>>,
    /// The transition public keys.
    tpk_map: DataMap<N::TransitionID, Group<N>>,
    /// The reverse `tpk` map.
    reverse_tpk_map: DataMap<Group<N>, N::TransitionID>,
    /// The transition commitments.
    tcm_map: DataMap<N::TransitionID, Field<N>>,
    /// The reverse `tcm` map.
    reverse_tcm_map: DataMap<Field<N>, N::TransitionID>,
    /// The transition fees.
    fee_map: DataMap<N::TransitionID, i64>,
}

#[rustfmt::skip]
impl<N: Network> TransitionStorage<N> for TransitionDB<N> {
    type LocatorMap = DataMap<N::TransitionID, (ProgramID<N>, Identifier<N>)>;
    type InputStorage = InputDB<N>;
    type OutputStorage = OutputDB<N>;
    type FinalizeMap = DataMap<N::TransitionID, Option<Vec<Value<N>>>>;
    type ProofMap = DataMap<N::TransitionID, Proof<N>>;
    type TPKMap = DataMap<N::TransitionID, Group<N>>;
    type ReverseTPKMap = DataMap<Group<N>, N::TransitionID>;
    type TCMMap = DataMap<N::TransitionID, Field<N>>;
    type ReverseTCMMap = DataMap<Field<N>, N::TransitionID>;
    type FeeMap = DataMap<N::TransitionID, i64>;

    /// Initializes the transition storage.
    fn open(dev: Option<u16>) -> Result<Self> {
        Ok(Self {
            locator_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::TransitionLocatorMap)?,
            input_store: InputStore::open(dev)?,
            output_store: OutputStore::open(dev)?,
            finalize_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::TransitionFinalizeMap)?,
            proof_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::TransitionProofMap)?,
            tpk_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::TransitionTPKMap)?,
            reverse_tpk_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::TransitionReverseTPKMap)?,
            tcm_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::TransitionTCMMap)?,
            reverse_tcm_map: rocksdb::RocksDB::open_map(N::ID, dev,  DataID::TransitionReverseTCMMap)?,
            fee_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::TransitionFeeMap)?,
        })
    }

    /// Returns the transition program IDs and function names.
    fn locator_map(&self) -> &Self::LocatorMap {
        &self.locator_map
    }

    /// Returns the transition input store.
    fn input_store(&self) -> &InputStore<N, Self::InputStorage> {
        &self.input_store
    }

    /// Returns the transition output store.
    fn output_store(&self) -> &OutputStore<N, Self::OutputStorage> {
        &self.output_store
    }

    /// Returns the transition finalize inputs map.
    fn finalize_map(&self) -> &Self::FinalizeMap {
        &self.finalize_map
    }

    /// Returns the transition proofs.
    fn proof_map(&self) -> &Self::ProofMap {
        &self.proof_map
    }

    /// Returns the transition public keys.
    fn tpk_map(&self) -> &Self::TPKMap {
        &self.tpk_map
    }

    /// Returns the reverse `tpk` map.
    fn reverse_tpk_map(&self) -> &Self::ReverseTPKMap {
        &self.reverse_tpk_map
    }

    /// Returns the transition commitments.
    fn tcm_map(&self) -> &Self::TCMMap {
        &self.tcm_map
    }

    /// Returns the reverse `tcm` map.
    fn reverse_tcm_map(&self) -> &Self::ReverseTCMMap {
        &self.reverse_tcm_map
    }

    /// Returns the transition fees.
    fn fee_map(&self) -> &Self::FeeMap {
        &self.fee_map
    }
}

/// An database transition input storage.
#[derive(Clone)]
pub struct InputDB<N: Network> {
    /// The mapping of `transition ID` to `input IDs`.
    id_map: DataMap<N::TransitionID, Vec<Field<N>>>,
    /// The mapping of `input ID` to `transition ID`.
    reverse_id_map: DataMap<Field<N>, N::TransitionID>,
    /// The mapping of `plaintext hash` to `(optional) plaintext`.
    constant: DataMap<Field<N>, Option<Plaintext<N>>>,
    /// The mapping of `plaintext hash` to `(optional) plaintext`.
    public: DataMap<Field<N>, Option<Plaintext<N>>>,
    /// The mapping of `ciphertext hash` to `(optional) ciphertext`.
    private: DataMap<Field<N>, Option<Ciphertext<N>>>,
    /// The mapping of `serial number` to `tag`.
    record: DataMap<Field<N>, Field<N>>,
    /// The mapping of `record tag` to `serial number`.
    record_tag: DataMap<Field<N>, Field<N>>,
    /// The mapping of `external commitment` to `()`. Note: This is **not** the record commitment.
    external_record: DataMap<Field<N>, ()>,
    /// The optional development ID.
    dev: Option<u16>,
}

#[rustfmt::skip]
impl<N: Network> InputStorage<N> for InputDB<N> {
    type IDMap = DataMap<N::TransitionID, Vec<Field<N>>>;
    type ReverseIDMap = DataMap<Field<N>, N::TransitionID>;
    type ConstantMap = DataMap<Field<N>, Option<Plaintext<N>>>;
    type PublicMap = DataMap<Field<N>, Option<Plaintext<N>>>;
    type PrivateMap = DataMap<Field<N>, Option<Ciphertext<N>>>;
    type RecordMap = DataMap<Field<N>, Field<N>>;
    type RecordTagMap = DataMap<Field<N>, Field<N>>;
    type ExternalRecordMap = DataMap<Field<N>, ()>;

    /// Initializes the transition input storage.
    fn open(dev: Option<u16>) -> Result<Self> {
        Ok(Self {
            id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::InputIDMap)?,
            reverse_id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::InputReverseIDMap)?,
            constant: rocksdb::RocksDB::open_map(N::ID, dev, DataID::InputConstantMap)?,
            public: rocksdb::RocksDB::open_map(N::ID, dev, DataID::InputPublicMap)?,
            private: rocksdb::RocksDB::open_map(N::ID, dev, DataID::InputPrivateMap)?,
            record: rocksdb::RocksDB::open_map(N::ID, dev, DataID::InputRecordMap)?,
            record_tag: rocksdb::RocksDB::open_map(N::ID, dev, DataID::InputRecordTagMap)?,
            external_record: rocksdb::RocksDB::open_map(N::ID, dev, DataID::InputExternalRecordMap)?,
            dev,
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

    /// Returns the constant map.
    fn constant_map(&self) -> &Self::ConstantMap {
        &self.constant
    }

    /// Returns the public map.
    fn public_map(&self) -> &Self::PublicMap {
        &self.public
    }

    /// Returns the private map.
    fn private_map(&self) -> &Self::PrivateMap {
        &self.private
    }

    /// Returns the record map.
    fn record_map(&self) -> &Self::RecordMap {
        &self.record
    }

    /// Returns the record tag map.
    fn record_tag_map(&self) -> &Self::RecordTagMap {
        &self.record_tag
    }

    /// Returns the external record map.
    fn external_record_map(&self) -> &Self::ExternalRecordMap {
        &self.external_record
    }

    /// Returns the optional development ID.
    fn dev(&self) -> Option<u16> {
        self.dev
    }
}

/// A database transition output storage.
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct OutputDB<N: Network> {
    /// The mapping of `transition ID` to `output IDs`.
    id_map: DataMap<N::TransitionID, Vec<Field<N>>>,
    /// The mapping of `output ID` to `transition ID`.
    reverse_id_map: DataMap<Field<N>, N::TransitionID>,
    /// The mapping of `plaintext hash` to `(optional) plaintext`.
    constant: DataMap<Field<N>, Option<Plaintext<N>>>,
    /// The mapping of `plaintext hash` to `(optional) plaintext`.
    public: DataMap<Field<N>, Option<Plaintext<N>>>,
    /// The mapping of `ciphertext hash` to `(optional) ciphertext`.
    private: DataMap<Field<N>, Option<Ciphertext<N>>>,
    /// The mapping of `commitment` to `(checksum, (optional) record ciphertext)`.
    record: DataMap<Field<N>, (Field<N>, Option<Record<N, Ciphertext<N>>>)>,
    /// The mapping of `record nonce` to `commitment`.
    record_nonce: DataMap<Group<N>, Field<N>>,
    /// The mapping of `external commitment` to `()`. Note: This is **not** the record commitment.
    external_record: DataMap<Field<N>, ()>,
    /// The optional development ID.
    dev: Option<u16>,
}

#[rustfmt::skip]
impl<N: Network> OutputStorage<N> for OutputDB<N> {
    type IDMap = DataMap<N::TransitionID, Vec<Field<N>>>;
    type ReverseIDMap = DataMap<Field<N>, N::TransitionID>;
    type ConstantMap = DataMap<Field<N>, Option<Plaintext<N>>>;
    type PublicMap = DataMap<Field<N>, Option<Plaintext<N>>>;
    type PrivateMap = DataMap<Field<N>, Option<Ciphertext<N>>>;
    type RecordMap = DataMap<Field<N>, (Field<N>, Option<Record<N, Ciphertext<N>>>)>;
    type RecordNonceMap = DataMap<Group<N>, Field<N>>;
    type ExternalRecordMap = DataMap<Field<N>, ()>;

    /// Initializes the transition output storage.
    fn open(dev: Option<u16>) -> Result<Self> {
        Ok(Self {
            id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::OutputIDMap)?,
            reverse_id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::OutputReverseIDMap)?,
            constant: rocksdb::RocksDB::open_map(N::ID, dev, DataID::OutputConstantMap)?,
            public: rocksdb::RocksDB::open_map(N::ID, dev, DataID::OutputPublicMap)?,
            private: rocksdb::RocksDB::open_map(N::ID, dev, DataID::OutputPrivateMap)?,
            record: rocksdb::RocksDB::open_map(N::ID, dev, DataID::OutputRecordMap)?,
            record_nonce: rocksdb::RocksDB::open_map(N::ID, dev, DataID::OutputRecordNonceMap)?,
            external_record: rocksdb::RocksDB::open_map(N::ID, dev, DataID::OutputExternalRecordMap)?,
            dev,
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

    /// Returns the constant map.
    fn constant_map(&self) -> &Self::ConstantMap {
        &self.constant
    }

    /// Returns the public map.
    fn public_map(&self) -> &Self::PublicMap {
        &self.public
    }

    /// Returns the private map.
    fn private_map(&self) -> &Self::PrivateMap {
        &self.private
    }

    /// Returns the record map.
    fn record_map(&self) -> &Self::RecordMap {
        &self.record
    }

    /// Returns the record nonce map.
    fn record_nonce_map(&self) -> &Self::RecordNonceMap {
        &self.record_nonce
    }

    /// Returns the external record map.
    fn external_record_map(&self) -> &Self::ExternalRecordMap {
        &self.external_record
    }

    /// Returns the optional development ID.
    fn dev(&self) -> Option<u16> {
        self.dev
    }
}
