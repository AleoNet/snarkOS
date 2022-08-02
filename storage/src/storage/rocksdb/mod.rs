pub mod data_map;
pub use data_map::*;

pub mod iterator;
use iterator::*;

pub mod keys;
use keys::*;

pub mod values;
use values::*;

use crate::storage::DataID;

#[cfg(test)]
mod tests;

use anyhow::Result;
use core::{fmt::Debug, hash::Hash};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    borrow::Borrow,
    convert::TryInto,
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    path::Path,
    sync::Arc,
};

pub const PREFIX_LEN: usize = 4; // N::ID (u16) + DataID (u16)

///
/// An instance of a RocksDB database.
///
#[derive(Clone)]
pub struct RocksDB {
    rocksdb: Arc<rocksdb::DB>,
    context: Vec<u8>,
}

impl RocksDB {
    pub fn inner(&self) -> &rocksdb::DB {
        &self.rocksdb
    }
}

impl RocksDB {
    ///
    /// Opens storage at the given `path` and `context`.
    ///
    fn open<P: AsRef<Path>>(path: P, context: u16) -> Result<Self> {
        let context = context.to_le_bytes().to_vec();

        // Customize database options.
        let mut options = rocksdb::Options::default();
        options.set_compression_type(rocksdb::DBCompressionType::Lz4);

        // Register the prefix length.
        let prefix_extractor = rocksdb::SliceTransform::create_fixed_prefix(PREFIX_LEN);
        options.set_prefix_extractor(prefix_extractor);

        let primary = path.as_ref().to_path_buf();
        let rocksdb = {
            options.increase_parallelism(2);
            options.create_if_missing(true);
            Arc::new(rocksdb::DB::open(&options, &primary)?)
        };

        Ok(RocksDB { rocksdb, context })
    }

    ///
    /// Opens a map with the given `context` from storage.
    ///
    fn open_map<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned>(&self, data_id: DataID) -> Result<DataMap<K, V>> {
        // Convert the new context into bytes.
        let new_context = (data_id as u16).to_le_bytes();

        // Combine contexts to create a new scope.
        let mut context_bytes = self.context.clone();
        context_bytes.extend_from_slice(&new_context);

        Ok(DataMap {
            storage: self.clone(),
            context: context_bytes,
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
}
