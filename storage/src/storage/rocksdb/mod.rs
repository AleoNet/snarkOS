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

mod iterator;
use iterator::*;

mod keys;
use keys::*;

mod map;
pub use map::*;

mod values;
use values::*;

#[cfg(test)]
mod tests;

use crate::storage::{Map, Storage};

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    borrow::Borrow,
    convert::TryInto,
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};

///
/// An instance of a RocksDB database.
///
#[derive(Clone)]
pub struct RocksDB {
    rocksdb: Arc<rocksdb::DB>,
    context: Vec<u8>,
    is_read_only: bool,
}

impl Storage for RocksDB {
    ///
    /// Opens storage at the given `path` and `context`.
    ///
    fn open<P: AsRef<Path>>(path: P, context: u16, is_read_only: bool) -> Result<Self> {
        let context = context.to_le_bytes().to_vec();

        // Customize database options.
        let mut options = rocksdb::Options::default();

        // Register the prefix length.
        let prefix_extractor = rocksdb::SliceTransform::create_fixed_prefix(PREFIX_LEN);
        options.set_prefix_extractor(prefix_extractor);

        let primary = path.as_ref().to_path_buf();
        let rocksdb = match is_read_only {
            true => {
                // Construct the directory paths.
                let reader = path.as_ref().join("reader");
                // Open a secondary reader for the primary rocksdb.
                let rocksdb = rocksdb::DB::open_as_secondary(&options, &primary, &reader)?;
                Arc::new(rocksdb)
            }
            false => {
                options.increase_parallelism(2);
                options.create_if_missing(true);
                Arc::new(rocksdb::DB::open(&options, &primary)?)
            }
        };

        let mut storage = RocksDB {
            rocksdb,
            context,
            is_read_only,
        };

        if !is_read_only {
            storage.migrate(path)?;
        }

        Ok(storage)
    }

    ///
    /// Opens a map with the given `context` from storage.
    ///
    fn open_map<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned>(&self, map_id: MapId) -> Result<DataMap<K, V>> {
        // Convert the new context into bytes.
        let new_context = (map_id as u16).to_le_bytes();

        // Combine contexts to create a new scope.
        let mut context_bytes = self.context.clone();
        context_bytes.extend_from_slice(&new_context);

        Ok(DataMap {
            rocksdb: self.rocksdb.clone(),
            context: context_bytes,
            is_read_only: self.is_read_only,
            _phantom: PhantomData,
        })
    }

    ///
    /// Imports a file with the given path to reconstruct storage.
    ///
    fn import<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let len = reader.seek(SeekFrom::End(0))?;
        reader.rewind()?;

        let mut buf = vec![0u8; 16 * 1024];

        while reader.stream_position()? < len {
            reader.read_exact(&mut buf[..4])?;
            let key_len = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;

            if key_len + 4 > buf.len() {
                buf.resize(key_len + 4, 0);
            }

            reader.read_exact(&mut buf[..key_len + 4])?;
            let value_len = u32::from_le_bytes(buf[key_len..][..4].try_into().unwrap()) as usize;

            if key_len + value_len > buf.len() {
                buf.resize(key_len + value_len, 0);
            }

            reader.read_exact(&mut buf[key_len..][..value_len])?;

            self.rocksdb.put(&buf[..key_len], &buf[key_len..][..value_len])?;
        }

        Ok(())
    }

    ///
    /// Exports the current state of storage to a single file at the specified location.
    ///
    fn export<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let mut iterator = self.rocksdb.raw_iterator();
        iterator.seek_to_first();

        while iterator.valid() {
            if let (Some(key), Some(value)) = (iterator.key(), iterator.value()) {
                writer.write_all(&(key.len() as u32).to_le_bytes())?;
                writer.write_all(key)?;

                writer.write_all(&(value.len() as u32).to_le_bytes())?;
                writer.write_all(value)?;
            }
            iterator.next();
        }

        Ok(())
    }

    ///
    /// Performs storage schema migration.
    ///
    fn migrate<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        // Check if migration is needed.
        let needs_prefix_shortening = {
            // This is the raw byte legacy format for the height of the genesis block.
            // This check transcends all `DataMap`s, as it is performed directly on the rocksdb instance.
            let key_with_long_prefix = vec![
                2, 0, 0, 0, 2, 0, 13, 0, 0, 0, 98, 108, 111, 99, 107, 95, 104, 101, 105, 103, 104, 116, 115, 0, 0, 0, 0,
            ];

            self.rocksdb.get(&key_with_long_prefix)?.is_some()
        };

        // An early return in case the db is empty or the schema is up to date.
        if !needs_prefix_shortening {
            debug!("The storage schema format is up to date");
            return Ok(());
        }

        debug!("The storage schema is out of date; performing migration");

        // Perform a backup of the whole storage at a neighboring location.
        let mut backup_path = path.as_ref().to_owned();
        backup_path.pop();
        backup_path = PathBuf::from(backup_path.to_string_lossy().into_owned());
        backup_path.set_extension("bak");
        debug!("Backing up storage at {}", backup_path.to_string_lossy());
        self.export(&backup_path)?;

        debug!("Migrating the storage to the new schema");
        // This is basically `Storage::import` which shortens the keys by removing the records with the legacy
        // format and inserting ones with the new one. It's not the fastest way to do it, but it's the safest.
        {
            let file = File::open(backup_path)?;
            let mut reader = BufReader::new(file);

            let len = reader.seek(SeekFrom::End(0))?;
            reader.rewind()?;

            let mut buf = vec![0u8; 16 * 1024];

            while reader.stream_position()? < len {
                reader.read_exact(&mut buf[..4])?;
                let key_len = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;

                if key_len + 4 > buf.len() {
                    buf.resize(key_len + 4, 0);
                }

                reader.read_exact(&mut buf[..key_len + 4])?;
                let value_len = u32::from_le_bytes(buf[key_len..][..4].try_into().unwrap()) as usize;

                if key_len + value_len > buf.len() {
                    buf.resize(key_len + value_len, 0);
                }

                reader.read_exact(&mut buf[key_len..][..value_len])?;

                // This part is where the code deviates from `Storage::import`.

                let old_key = &buf[..key_len];
                let value = &buf[key_len..][..value_len];

                // Remember the N::NETWORK_ID for later.
                let network_id = &old_key[4..][..2];

                // Remove the N::NETWORK_ID bits and the label length.
                if old_key.len() <= 10 {
                    // The storage iterator can stumble across the updated records; ignore them.
                    continue;
                }
                let mut old_key_shortened = &old_key[10..];

                // Determine which map the record belongs to.
                let mut new_map_id = None;

                for (i, label) in [
                    &b"block_headers"[..],
                    &b"block_heights"[..],
                    &b"block_transactions"[..],
                    &b"commitments"[..],
                    &b"ledger_roots"[..],
                    &b"records"[..],
                    &b"serial_numbers"[..],
                    &b"transactions"[..],
                    &b"transitions"[..],
                    &b"shares"[..],
                ]
                .iter()
                .enumerate()
                {
                    if old_key_shortened.starts_with(label) {
                        new_map_id = Some(i as u16);
                        old_key_shortened = &old_key_shortened[label.len()..];
                    }
                }

                let new_map_id = if let Some(id) = new_map_id {
                    id
                } else {
                    // The storage iterator can stumble across the updated records; ignore them.
                    continue;
                };

                let mut new_key = network_id.to_vec();
                new_key.extend_from_slice(&new_map_id.to_le_bytes());
                new_key.extend_from_slice(old_key_shortened);

                self.rocksdb.put(&new_key, value)?;
                self.rocksdb.delete(old_key)?;
            }
        }

        Ok(())
    }
}
