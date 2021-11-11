// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use crate::storage::{rocksdb::RocksDB, Map, Storage};

fn temp_dir() -> std::path::PathBuf {
    tempfile::tempdir().expect("Failed to open temporary directory").into_path()
}

#[test]
fn test_open() {
    let _storage = RocksDB::open(temp_dir(), 0, false).expect("Failed to open storage");
}

#[test]
fn test_open_map() {
    let storage = RocksDB::open(temp_dir(), 0, false).expect("Failed to open storage");
    storage.open_map::<u32, String>("hello world").expect("Failed to open data map");
}

#[test]
fn test_insert_and_contains_key() {
    let storage = RocksDB::open(temp_dir(), 0, false).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>("hello world").expect("Failed to open data map");

    map.insert(&123456789, &"123456789".to_string()).expect("Failed to insert");
    assert!(map.contains_key(&123456789).expect("Failed to call contains key"));
    assert!(!map.contains_key(&000000000).expect("Failed to call contains key"));
}

#[test]
fn test_insert_and_get() {
    let storage = RocksDB::open(temp_dir(), 0, false).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>("hello world").expect("Failed to open data map");

    map.insert(&123456789, &"123456789".to_string()).expect("Failed to insert");
    assert_eq!(Some("123456789".to_string()), map.get(&123456789).expect("Failed to get"));
    assert_eq!(None, map.get(&000000000).expect("Failed to get"));
}

#[test]
fn test_insert_and_remove() {
    let storage = RocksDB::open(temp_dir(), 0, false).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>("hello world").expect("Failed to open data map");

    map.insert(&123456789, &"123456789".to_string()).expect("Failed to insert");
    assert!(map.get(&123456789).expect("Failed to get").is_some());

    map.remove(&123456789).expect("Failed to remove");
    assert!(map.get(&123456789).expect("Failed to get").is_none());
}

#[test]
fn test_insert_and_iter() {
    let storage = RocksDB::open(temp_dir(), 0, false).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>("hello world").expect("Failed to open data map");
    map.insert(&123456789, &"123456789".to_string()).expect("Failed to insert");

    let mut iter = map.iter();
    assert_eq!(Some((123456789, "123456789".to_string())), iter.next());
    assert_eq!(None, iter.next());
}

#[test]
fn test_insert_and_keys() {
    let storage = RocksDB::open(temp_dir(), 0, false).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>("hello world").expect("Failed to open data map");
    map.insert(&123456789, &"123456789".to_string()).expect("Failed to insert");

    let mut keys = map.keys();
    assert_eq!(Some(123456789), keys.next());
    assert_eq!(None, keys.next());
}

#[test]
fn test_insert_and_values() {
    let storage = RocksDB::open(temp_dir(), 0, false).expect("Failed to open storage");
    let map = storage.open_map::<u32, String>("hello world").expect("Failed to open data map");
    map.insert(&123456789, &"123456789".to_string()).expect("Failed to insert");

    let mut values = map.values();
    assert_eq!(Some("123456789".to_string()), values.next());
    assert_eq!(None, values.next());
}

#[test]
fn test_reopen() {
    let directory = temp_dir();
    {
        let storage = RocksDB::open(directory.clone(), 0, false).expect("Failed to open storage");
        let map = storage.open_map::<u32, String>("hello world").expect("Failed to open data map");
        map.insert(&123456789, &"123456789".to_string()).expect("Failed to insert");
        drop(storage);
    }
    {
        let storage = RocksDB::open(directory, 0, false).expect("Failed to open storage");
        let map = storage.open_map::<u32, String>("hello world").expect("Failed to open data map");
        assert_eq!(Some("123456789".to_string()), map.get(&123456789).expect("Failed to get"));
    }
}
