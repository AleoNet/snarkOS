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

use crate::storage::{rocksdb::RocksDB, MapId, MapRead, MapReadWrite, ReadWrite, Storage};

fn temp_dir() -> std::path::PathBuf {
    tempfile::tempdir().expect("Failed to open temporary directory").into_path()
}

fn temp_file() -> std::path::PathBuf {
    tempfile::NamedTempFile::new()
        .expect("Failed to open temporary file")
        .path()
        .to_owned()
}

#[test]
fn test_open() {
    let _storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
}

#[test]
fn test_open_map() {
    let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
    storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");
}

#[test]
fn test_insert_and_contains_key() {
    let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");

    map.insert(&123456789, &"123456789".to_string(), None).expect("Failed to insert");
    assert!(map.contains_key(&123456789).expect("Failed to call contains key"));
    assert!(!map.contains_key(&000000000).expect("Failed to call contains key"));
}

#[test]
fn test_insert_and_get() {
    let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");

    map.insert(&123456789, &"123456789".to_string(), None).expect("Failed to insert");
    assert_eq!(Some("123456789".to_string()), map.get(&123456789).expect("Failed to get"));
    assert_eq!(None, map.get(&000000000).expect("Failed to get"));
}

#[test]
fn test_insert_and_remove() {
    let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");

    map.insert(&123456789, &"123456789".to_string(), None).expect("Failed to insert");
    assert!(map.get(&123456789).expect("Failed to get").is_some());

    map.remove(&123456789, None).expect("Failed to remove");
    assert!(map.get(&123456789).expect("Failed to get").is_none());
}

#[test]
fn test_insert_and_iter() {
    let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");
    map.insert(&123456789, &"123456789".to_string(), None).expect("Failed to insert");

    let mut iter = map.iter();
    assert_eq!(Some((123456789, "123456789".to_string())), iter.next());
    assert_eq!(None, iter.next());
}

#[test]
fn test_insert_and_keys() {
    let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");
    map.insert(&123456789, &"123456789".to_string(), None).expect("Failed to insert");

    let mut keys = map.keys();
    assert_eq!(Some(123456789), keys.next());
    assert_eq!(None, keys.next());
}

#[test]
fn test_insert_and_values() {
    let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");
    map.insert(&123456789, &"123456789".to_string(), None).expect("Failed to insert");

    let mut values = map.values();
    assert_eq!(Some("123456789".to_string()), values.next());
    assert_eq!(None, values.next());
}

#[test]
fn test_reopen() {
    let directory = temp_dir();
    {
        let storage = RocksDB::<ReadWrite>::open(directory.clone(), 0).expect("Failed to open storage");
        let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");
        map.insert(&123456789, &"123456789".to_string(), None).expect("Failed to insert");
    }
    {
        let storage = RocksDB::<ReadWrite>::open(directory, 0).expect("Failed to open storage");
        let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");
        assert_eq!(Some("123456789".to_string()), map.get(&123456789).expect("Failed to get"));
    }
}

#[test]
fn test_batch_insert_and_remove() {
    let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");

    let batch = map.prepare_batch();

    map.insert(&1, &"1".to_string(), Some(batch)).expect("Failed to insert");
    assert!(map.get(&1).expect("Failed to get").is_none());

    map.insert(&2, &"2".to_string(), Some(batch)).expect("Failed to insert");
    assert!(map.get(&2).expect("Failed to get").is_none());

    map.execute_batch(batch).expect("Failed to execute a batch");
    assert!(map.get(&1).expect("Failed to get").is_some());
    assert!(map.get(&2).expect("Failed to get").is_some());
    assert!(map.execute_batch(batch).is_err());

    let batch = map.prepare_batch();

    map.remove(&1, Some(batch)).expect("Failed to remove");
    assert!(map.get(&1).expect("Failed to get").is_some());
    map.remove(&2, Some(batch)).expect("Failed to remove");
    assert!(map.get(&2).expect("Failed to get").is_some());

    map.execute_batch(batch).expect("Failed to execute a batch");
    assert!(map.get(&1).expect("Failed to get").is_none());
    assert!(map.get(&2).expect("Failed to get").is_none());
    assert!(map.execute_batch(batch).is_err());
}

#[test]
fn test_multiple_batches() {
    let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");

    let batch1 = map.prepare_batch();
    let batch2 = map.prepare_batch();
    let batch3 = map.prepare_batch();

    map.insert(&1, &"1".to_string(), Some(batch1)).expect("Failed to insert");
    map.insert(&2, &"2".to_string(), Some(batch1)).expect("Failed to insert");

    map.insert(&3, &"3".to_string(), Some(batch2)).expect("Failed to insert");
    map.insert(&4, &"4".to_string(), Some(batch2)).expect("Failed to insert");

    map.insert(&5, &"5".to_string(), Some(batch3)).expect("Failed to insert");
    map.insert(&6, &"6".to_string(), Some(batch3)).expect("Failed to insert");

    for i in 1..=6 {
        assert!(map.get(&i).expect("Failed to get").is_none());
    }

    map.execute_batch(batch3).expect("Failed to execute a batch");
    assert!(map.get(&5).expect("Failed to get").is_some());
    assert!(map.get(&6).expect("Failed to get").is_some());
    assert!(map.execute_batch(batch3).is_err());

    for i in 1..=4 {
        assert!(map.get(&i).expect("Failed to get").is_none());
    }

    map.execute_batch(batch2).expect("Failed to execute a batch");
    assert!(map.get(&3).expect("Failed to get").is_some());
    assert!(map.get(&4).expect("Failed to get").is_some());
    assert!(map.execute_batch(batch2).is_err());

    assert!(map.get(&1).expect("Failed to get").is_none());
    assert!(map.get(&2).expect("Failed to get").is_none());
}

#[test]
fn test_discard_batch() {
    let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");

    let batch = map.prepare_batch();

    map.insert(&1, &"1".to_string(), Some(batch)).expect("Failed to insert");
    map.insert(&2, &"2".to_string(), Some(batch)).expect("Failed to insert");

    assert!(map.get(&1).expect("Failed to get").is_none());
    assert!(map.get(&2).expect("Failed to get").is_none());

    assert!(map.discard_batch(batch).is_ok());
    assert!(map.execute_batch(batch).is_err());

    assert!(map.get(&1).expect("Failed to get").is_none());
    assert!(map.get(&2).expect("Failed to get").is_none());
}

#[test]
fn test_export_import() {
    let file = temp_file();

    {
        let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
        let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");

        for i in 0..100 {
            map.insert(&i, &i.to_string(), None).expect("Failed to insert");
        }

        storage.export(&file).expect("Failed to export storage");
    }

    let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
    storage.import(&file).expect("Failed to import storage");

    let map = storage.open_map::<u32, String>(MapId::Test).expect("Failed to open data map");

    for i in 0..100 {
        assert_eq!(map.get(&i).expect("Failed to get"), Some(i.to_string()));
    }
}
