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
    rocksdb::{DataMap, RocksDB},
    DataID,
};
use snarkvm::{
    console::{network::Testnet3, types::Scalar},
    prelude::{Network, TestRng, Uniform},
    synthesizer::store::helpers::{Map, MapRead},
};

use serial_test::serial;
use std::borrow::Cow;

type TestMap = DataMap<u32, String>;

pub(crate) fn temp_dir() -> std::path::PathBuf {
    tempfile::tempdir().expect("Failed to open temporary directory").into_path()
}

// pub(crate) fn temp_file() -> std::path::PathBuf {
//     tempfile::NamedTempFile::new().expect("Failed to open temporary file").path().to_owned()
// }

#[test]
#[serial]
fn test_open() {
    let _storage = RocksDB::open_testing(temp_dir(), None).expect("Failed to open storage");
}

#[test]
#[serial]
fn test_open_map() {
    let _map =
        RocksDB::open_map_testing::<u32, String>(temp_dir(), None, DataID::Test).expect("Failed to open data map");
}

#[test]
#[serial]
fn test_insert_and_contains_key() {
    let map = RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");

    map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    assert!(map.contains_key(&123456789).expect("Failed to call contains key"));
    assert!(!map.contains_key(&000000000).expect("Failed to call contains key"));
}

#[test]
#[serial]
fn test_insert_and_get() {
    let map = RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");

    map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    assert_eq!(Some("123456789".to_string()), map.get(&123456789).expect("Failed to get").map(|v| v.to_string()));

    assert_eq!(None, map.get(&000000000).expect("Failed to get"));
}

#[test]
#[serial]
fn test_insert_and_remove() {
    let map = RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");

    map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    assert_eq!(map.get(&123456789).expect("Failed to get").map(|v| v.to_string()), Some("123456789".to_string()));

    map.remove(&123456789).expect("Failed to remove");
    assert!(map.get(&123456789).expect("Failed to get").is_none());
}

#[test]
#[serial]
fn test_insert_and_iter() {
    let map = RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");

    map.insert(123456789, "123456789".to_string()).expect("Failed to insert");

    let mut iter = map.iter();
    assert_eq!(Some((123456789, "123456789".to_string())), iter.next().map(|(k, v)| (*k, v.to_string())));
    assert_eq!(None, iter.next());
}

#[test]
#[serial]
fn test_insert_and_keys() {
    let map = RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");

    map.insert(123456789, "123456789".to_string()).expect("Failed to insert");

    let mut keys = map.keys();
    assert_eq!(Some(123456789), keys.next().map(|k| *k));
    assert_eq!(None, keys.next());
}

#[test]
#[serial]
fn test_insert_and_values() {
    let map = RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");

    map.insert(123456789, "123456789".to_string()).expect("Failed to insert");

    let mut values = map.values();
    assert_eq!(Some("123456789".to_string()), values.next().map(|v| v.to_string()));
    assert_eq!(None, values.next());
}

#[test]
#[serial]
fn test_reopen() {
    let directory = temp_dir();
    {
        let map = RocksDB::open_map_testing(directory.clone(), None, DataID::Test).expect("Failed to open data map");
        map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    }
    {
        let map: TestMap = RocksDB::open_map_testing(directory, None, DataID::Test).expect("Failed to open data map");
        match map.get(&123456789).expect("Failed to get") {
            Some(Cow::Borrowed(value)) => assert_eq!(value.to_string(), "123456789".to_string()),
            Some(Cow::Owned(value)) => assert_eq!(value, "123456789".to_string()),
            None => panic!("Failed to get value"),
        }
    }
}

// #[test]
// #[serial]
// fn test_export_import() {
//     let file = temp_file();
//     {
//         let mut map = RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");
//
//         for i in 0..100 {
//             map.insert(i, i.to_string()).expect("Failed to insert");
//         }
//
//         storage.export(&file).expect("Failed to export storage");
//     }
//
//     let storage = RocksDB::open_temporary(temp_dir(), None).expect("Failed to open storage");
//     storage.import(&file).expect("Failed to import storage");
//
//     let map = storage.open_map::<u32, String>(DataID::Test).expect("Failed to open data map");
//
//     for i in 0..100 {
//         assert_eq!(map.get(&i).expect("Failed to get").map(|v| v.to_string()), Some(i.to_string()));
//     }
// }

#[test]
#[serial]
fn test_scalar_mul() {
    type CurrentNetwork = Testnet3;

    let rng = &mut TestRng::default();

    const ITERATIONS: u32 = 1_000_000u32;

    let map = RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");

    // Sample `ITERATION` random field elements to store.
    for i in 0..ITERATIONS {
        let value = Scalar::<CurrentNetwork>::rand(rng);
        map.insert(i, value).expect("Failed to insert");
    }

    let timer = std::time::Instant::now();

    // Execute scalar multiplication for each stored element.
    for value in map.values() {
        let _group = CurrentNetwork::g_scalar_multiply(&*value);
    }

    let elapsed = timer.elapsed().as_secs();
    println!(" {ITERATIONS} Scalar Muls : {elapsed} s");
}
