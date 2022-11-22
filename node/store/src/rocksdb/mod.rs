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

pub mod map;
pub use map::*;

pub mod iterator;
use iterator::*;

#[cfg(test)]
mod tests;

use crate::DataID;

use anyhow::{bail, Result};
use core::{fmt::Debug, hash::Hash};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    borrow::Borrow,
    marker::PhantomData,
    ops::Deref,
    sync::{atomic::AtomicBool, Arc},
};

pub const PREFIX_LEN: usize = 4; // N::ID (u16) + DataID (u16)

pub trait Database {
    /// Opens the database.
    fn open(network_id: u16, dev: Option<u16>) -> Result<Self>
    where
        Self: Sized;

    /// Opens the map with the given `network_id`, `(optional) development ID`, and `data_id` from storage.
    fn open_map<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned>(
        network_id: u16,
        dev: Option<u16>,
        data_id: DataID,
    ) -> Result<DataMap<K, V>>;
}

/// An instance of a RocksDB database.
#[derive(Clone)]
pub struct RocksDB {
    /// The RocksDB instance.
    rocksdb: Arc<rocksdb::DB>,
    /// The network ID.
    network_id: u16,
    /// The optional development ID.
    dev: Option<u16>,
}

impl Deref for RocksDB {
    type Target = Arc<rocksdb::DB>;

    fn deref(&self) -> &Self::Target {
        &self.rocksdb
    }
}

impl Database for RocksDB {
    /// Opens the database.
    ///
    /// In production mode, the database opens directory `~/.aleo/storage/ledger-{network}`.
    /// In development mode, the database opens directory `/path/to/repo/.ledger-{network}-{id}`.
    fn open(network_id: u16, dev: Option<u16>) -> Result<Self> {
        static DB: OnceCell<RocksDB> = OnceCell::new();

        // Retrieve the database.
        let database = DB
            .get_or_try_init(|| {
                // Customize database options.
                let mut options = rocksdb::Options::default();
                options.set_compression_type(rocksdb::DBCompressionType::Lz4);

                // Register the prefix length.
                let prefix_extractor = rocksdb::SliceTransform::create_fixed_prefix(PREFIX_LEN);
                options.set_prefix_extractor(prefix_extractor);

                let primary = aleo_std::aleo_ledger_dir(network_id, dev);
                let rocksdb = {
                    options.increase_parallelism(2);
                    options.create_if_missing(true);

                    Arc::new(rocksdb::DB::open(&options, primary)?)
                };

                Ok::<_, anyhow::Error>(RocksDB { rocksdb, network_id, dev })
            })?
            .clone();

        // Ensure the database network ID and development ID match.
        match database.network_id == network_id && database.dev == dev {
            true => Ok(database),
            false => bail!("Mismatching network ID or development ID in the database"),
        }
    }

    /// Opens the map with the given `network_id`, `(optional) development ID`, and `data_id` from storage.
    fn open_map<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned>(
        network_id: u16,
        dev: Option<u16>,
        data_id: DataID,
    ) -> Result<DataMap<K, V>> {
        // Open the RocksDB database.
        let database = Self::open(network_id, dev)?;

        // Combine contexts to create a new scope.
        let mut context = database.network_id.to_le_bytes().to_vec();
        context.extend_from_slice(&(data_id as u16).to_le_bytes());

        // Return the DataMap.
        Ok(DataMap { database, context, batch_in_progress: Default::default(), atomic_batch: Default::default() })
    }
}

impl RocksDB {
    /// Opens the test database.
    #[cfg(test)]
    fn open_testing(temp_dir: std::path::PathBuf, dev: Option<u16>) -> Result<Self> {
        let database = {
            // Customize database options.
            let mut options = rocksdb::Options::default();
            options.set_compression_type(rocksdb::DBCompressionType::Lz4);

            // Register the prefix length.
            let prefix_extractor = rocksdb::SliceTransform::create_fixed_prefix(PREFIX_LEN);
            options.set_prefix_extractor(prefix_extractor);

            // Construct the directory for the test database.
            let primary = match dev {
                Some(dev) => temp_dir.join(dev.to_string()),
                None => temp_dir,
            };

            let rocksdb = {
                options.increase_parallelism(2);
                options.create_if_missing(true);
                Arc::new(rocksdb::DB::open(&options, primary)?)
            };

            Ok::<_, anyhow::Error>(RocksDB { rocksdb, network_id: u16::MAX, dev })
        }?;

        // Ensure the database development ID match.
        match database.dev == dev {
            true => Ok(database),
            false => bail!("Mismatching development ID in the test database"),
        }
    }

    /// Opens the test map.
    #[cfg(test)]
    fn open_map_testing<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned>(
        temp_dir: std::path::PathBuf,
        dev: Option<u16>,
        data_id: DataID,
    ) -> Result<DataMap<K, V>> {
        // Open the RocksDB test database.
        let database = Self::open_testing(temp_dir, dev)?;

        // Combine contexts to create a new scope.
        let mut context = database.network_id.to_le_bytes().to_vec();
        context.extend_from_slice(&(data_id as u16).to_le_bytes());

        // Return the DataMap.
        Ok(DataMap { database, context, batch_in_progress: Default::default(), atomic_batch: Default::default() })
    }
}

// impl RocksDB {
//     /// Imports a file with the given path to reconstruct storage.
//     fn import<P: AsRef<Path>>(&self, path: P) -> Result<()> {
//         let file = File::open(path)?;
//         let mut reader = BufReader::new(file);
//
//         let len = reader.seek(SeekFrom::End(0))?;
//         reader.rewind()?;
//
//         let mut buf = vec![0u8; 16 * 1024];
//
//         while reader.stream_position()? < len {
//             reader.read_exact(&mut buf[..4])?;
//             let key_len = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
//
//             if key_len + 4 > buf.len() {
//                 buf.resize(key_len + 4, 0);
//             }
//
//             reader.read_exact(&mut buf[..key_len + 4])?;
//             let value_len = u32::from_le_bytes(buf[key_len..][..4].try_into().unwrap()) as usize;
//
//             if key_len + value_len > buf.len() {
//                 buf.resize(key_len + value_len, 0);
//             }
//
//             reader.read_exact(&mut buf[key_len..][..value_len])?;
//
//             self.rocksdb.put(&buf[..key_len], &buf[key_len..][..value_len])?;
//         }
//
//         Ok(())
//     }
//
//     /// Exports the current state of storage to a single file at the specified location.
//     fn export<P: AsRef<Path>>(&self, path: P) -> Result<()> {
//         let file = File::create(path)?;
//         let mut writer = BufWriter::new(file);
//
//         let mut iterator = self.rocksdb.raw_iterator();
//         iterator.seek_to_first();
//
//         while iterator.valid() {
//             if let (Some(key), Some(value)) = (iterator.key(), iterator.value()) {
//                 writer.write_all(&(key.len() as u32).to_le_bytes())?;
//                 writer.write_all(key)?;
//
//                 writer.write_all(&(value.len() as u32).to_le_bytes())?;
//                 writer.write_all(value)?;
//             }
//             iterator.next();
//         }
//
//         Ok(())
//     }
// }
