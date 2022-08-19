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
    store::rocksdb::{DataMap, Database, RocksDB},
    DataID,
};
use snarkvm::compiler::{Map, MapRead};

use std::fs;

fn remove_test_dir(network_id: u16) {
    let _ = fs::remove_dir_all(format!("~/.aleo/storage/.ledger-{}", network_id));
}

fn setup_test_map(network_id: u16) -> TestMap {
    remove_test_dir(network_id);
    RocksDB::open_map(network_id, DataID::Test).expect("Failed to open data map")
}

type TestMap = DataMap<u32, String>;

#[test]
fn test_open() {
    remove_test_dir(0);
    assert!(RocksDB::open(0).is_ok());
}

#[test]
fn test_open_map_opens_a_datamap_for_a_given_dataid() {
    remove_test_dir(1);
    assert!(RocksDB::open_map::<i32, String>(1, DataID::Test).is_ok());
}

#[test]
fn test_a_key_is_contained_after_a_value_was_inserted_with_it() {
    let test_map = setup_test_map(2);

    test_map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    let expected_result = test_map.contains_key(&123456789);

    assert!(expected_result.is_ok());
}

#[test]
fn test_a_key_is_not_contained_if_no_value_was_inserted_with_it() {
    let test_map = setup_test_map(3);

    assert!(!test_map.contains_key(&000000000).expect("Failed to call contains key"));
}

#[test]
fn test_a_value_that_was_inserted_can_be_retrieved_with_its_associated_key() {
    let test_map = setup_test_map(4);

    test_map.insert(123456789, "123456789".to_string()).expect("Failed to insert");

    assert_eq!(
        Some("123456789".to_string()),
        test_map.get(&123456789).expect("Failed to get").map(|v| v.to_string())
    );
}

#[test]
fn test_trying_to_get_a_value_associated_to_a_non_existent_key_returns_none() {
    let test_map = setup_test_map(5);

    let expected_result = test_map.get(&000000000).expect("Failed to get");

    assert!(expected_result.is_none());
}

#[test]
fn test_a_value_that_was_inserted_can_be_removed_with_its_associated_key() {
    let test_map = setup_test_map(6);
    test_map.insert(123456789, "123456789".to_string()).expect("Failed to insert");

    test_map.remove(&123456789).expect("Failed to remove");
    let expected_result = test_map.get(&123456789).expect("Failed to get");

    assert!(expected_result.is_none());
}

#[test]
#[ignore = "Removing a key that does not exist does not result in an error"]
fn test_a_value_that_was_not_inserted_cannot_be_removed() {
    let test_map = setup_test_map(7);

    let expected_result = test_map.remove(&123456789);

    assert!(expected_result.is_err());
}

#[test]
#[ignore = "Removing a key that does not exist does not result in an error"]
fn test_a_value_cannot_be_removed_twice() {
    let test_map = setup_test_map(8);
    test_map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    test_map.remove(&123456789).expect("Failed to remove");

    let expected_result = test_map.remove(&123456789);

    assert!(expected_result.is_err());
}

#[test]
fn test_can_iter_on_pairs_after_inserting() {
    let test_map = setup_test_map(9);

    test_map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    let expected_result = test_map.iter().next().map(|(k, v)| (*k, v.to_string()));

    assert_eq!(Some((123456789, "123456789".to_string())), expected_result);
}

#[test]
fn test_can_iter_on_keys_after_inserting() {
    let test_map = setup_test_map(11);

    test_map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    let mut keys = test_map.keys();
    let expected_result = keys.next().map(|k| *k);
    let expected_none = keys.next();

    assert_eq!(Some(123456789), expected_result);
    assert!(expected_none.is_none());
}

#[test]
fn test_can_iter_on_values_after_inserting() {
    let test_map = setup_test_map(12);

    test_map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    let mut values = test_map.values();
    let expected_result = values.next().map(|v| v.to_string());
    let expected_none = values.next();

    assert_eq!(Some("123456789".to_string()), expected_result);
    assert!(expected_none.is_none());
}

#[test]
fn test_reopen() {
    {
        let test_map = setup_test_map(13);
        test_map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    }
    {
        let test_map: TestMap = RocksDB::open_map(13, DataID::Test).expect("Failed to open data map");
        let expected_result = test_map.get(&123456789).expect("Failed to get").map(|v| v.to_string());

        remove_test_dir(13);

        assert_eq!(Some("123456789".to_string()), expected_result);
    }
}
