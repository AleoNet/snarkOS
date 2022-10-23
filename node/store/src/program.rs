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

use indexmap::{IndexMap, IndexSet};

/// A RocksDB program storage.
#[derive(Clone)]
pub struct ProgramDB<N: Network> {
    /// The program ID map.
    program_id_map: DataMap<ProgramID<N>, IndexSet<Identifier<N>>>,
    /// The mapping ID map.
    mapping_id_map: DataMap<(ProgramID<N>, Identifier<N>), Field<N>>,
    /// The key-value ID map.
    key_value_id_map: DataMap<Field<N>, IndexMap<Field<N>, Field<N>>>,
    /// The key map.
    key_map: DataMap<Field<N>, Plaintext<N>>,
    /// The value map.
    value_map: DataMap<Field<N>, Value<N>>,
    /// The optional development ID.
    dev: Option<u16>,
}

#[rustfmt::skip]
impl<N: Network> ProgramStorage<N> for ProgramDB<N> {
    type ProgramIDMap = DataMap<ProgramID<N>, IndexSet<Identifier<N>>>;
    type MappingIDMap = DataMap<(ProgramID<N>, Identifier<N>), Field<N>>;
    type KeyValueIDMap = DataMap<Field<N>, IndexMap<Field<N>, Field<N>>>;
    type KeyMap = DataMap<Field<N>, Plaintext<N>>;
    type ValueMap = DataMap<Field<N>, Value<N>>;

    /// Initializes the program state storage.
    fn open(dev: Option<u16>) -> Result<Self> {
        Ok(Self {
            program_id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::ProgramIDMap)?,
            mapping_id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::MappingIDMap)?,
            key_value_id_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::KeyValueIDMap)?,
            key_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::KeyMap)?,
            value_map: rocksdb::RocksDB::open_map(N::ID, dev, DataID::ValueMap)?,
            dev,
        })
    }

    /// Returns the program ID map.
    fn program_id_map(&self) -> &Self::ProgramIDMap {
        &self.program_id_map
    }

    /// Returns the mapping ID map.
    fn mapping_id_map(&self) -> &Self::MappingIDMap {
        &self.mapping_id_map
    }

    /// Returns the key-value ID map.
    fn key_value_id_map(&self) -> &Self::KeyValueIDMap {
        &self.key_value_id_map
    }

    /// Returns the key map.
    fn key_map(&self) -> &Self::KeyMap {
        &self.key_map
    }

    /// Returns the value map.
    fn value_map(&self) -> &Self::ValueMap {
        &self.value_map
    }

    /// Returns the optional development ID.
    fn dev(&self) -> Option<u16> {
        self.dev
    }
}
