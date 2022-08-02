use crate::storage::{rocksdb::RocksDB, DataID, DataMap};

use snarkvm::compiler::{Map, MapReader};

pub(crate) fn temp_dir() -> std::path::PathBuf {
    tempfile::tempdir().expect("Failed to open temporary directory").into_path()
}

pub(crate) fn temp_file() -> std::path::PathBuf {
    tempfile::NamedTempFile::new()
        .expect("Failed to open temporary file")
        .path()
        .to_owned()
}

#[test]
fn test_open() {
    let _storage = RocksDB::open(temp_dir(), 0).expect("Failed to open storage");
}

#[test]
fn test_open_map() {
    let storage = RocksDB::open(temp_dir(), 0).expect("Failed to open storage");
    storage.open_map::<u32, String>(DataID::Test).expect("Failed to open data map");
}

#[test]
fn test_insert_and_contains_key() {
    let storage = RocksDB::open(temp_dir(), 0).expect("Failed to open storage");
    let mut map = storage.open_map::<u32, String>(DataID::Test).expect("Failed to open data map");

    map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    assert!(map.contains_key(&123456789).expect("Failed to call contains key"));
    assert!(!map.contains_key(&000000000).expect("Failed to call contains key"));
}

#[test]
fn test_insert_and_get() {
    let storage = RocksDB::open(temp_dir(), 0).expect("Failed to open storage");
    let mut map = storage.open_map::<u32, String>(DataID::Test).expect("Failed to open data map");

    map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    assert_eq!(
        Some("123456789".to_string()),
        map.get(&123456789).expect("Failed to get").map(|v| v.to_string())
    );

    assert_eq!(None, map.get(&000000000).expect("Failed to get"));
}

#[test]
fn test_insert_and_remove() {
    let storage = RocksDB::open(temp_dir(), 0).expect("Failed to open storage");
    let mut map = storage.open_map::<u32, String>(DataID::Test).expect("Failed to open data map");

    map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    assert_eq!(
        map.get(&123456789).expect("Failed to get").map(|v| v.to_string()),
        Some("123456789".to_string())
    );

    map.remove(&123456789).expect("Failed to remove");
    assert!(map.get(&123456789).expect("Failed to get").is_none());
}

#[test]
fn test_insert_and_iter() {
    let storage = RocksDB::open(temp_dir(), 0).expect("Failed to open storage");
    let mut map = storage.open_map::<u32, String>(DataID::Test).expect("Failed to open data map");
    map.insert(123456789, "123456789".to_string()).expect("Failed to insert");

    let mut iter = map.iter();
    assert_eq!(
        Some((123456789, "123456789".to_string())),
        iter.next().map(|(k, v)| (*k, v.to_string()))
    );
    assert_eq!(None, iter.next());
}

#[test]
fn test_insert_and_keys() {
    let storage = RocksDB::open(temp_dir(), 0).expect("Failed to open storage");
    let mut map = storage.open_map::<u32, String>(DataID::Test).expect("Failed to open data map");
    map.insert(123456789, "123456789".to_string()).expect("Failed to insert");

    let mut keys = map.keys();
    assert_eq!(Some(123456789), keys.next().map(|k| *k));
    assert_eq!(None, keys.next());
}

#[test]
fn test_insert_and_values() {
    let storage = RocksDB::open(temp_dir(), 0).expect("Failed to open storage");
    let mut map = storage.open_map::<u32, String>(DataID::Test).expect("Failed to open data map");
    map.insert(123456789, "123456789".to_string()).expect("Failed to insert");

    let mut values = map.values();
    assert_eq!(Some("123456789".to_string()), values.next().map(|v| v.to_string()));
    assert_eq!(None, values.next());
}

#[test]
fn test_reopen() {
    let directory = temp_dir();
    {
        let storage = RocksDB::open(directory.clone(), 0).expect("Failed to open storage");
        let mut map = storage.open_map::<u32, String>(DataID::Test).expect("Failed to open data map");
        map.insert(123456789, "123456789".to_string()).expect("Failed to insert");
    }
    {
        let storage = RocksDB::open(directory, 0).expect("Failed to open storage");
        let map = storage.open_map::<u32, String>(DataID::Test).expect("Failed to open data map");
        assert_eq!(
            Some("123456789".to_string()),
            map.get(&123456789).expect("Failed to get").map(|v| v.to_string())
        );
    }
}

#[test]
fn test_export_import() {
    let file = temp_file();

    {
        let storage = RocksDB::open(temp_dir(), 0).expect("Failed to open storage");
        let mut map = storage.open_map::<u32, String>(DataID::Test).expect("Failed to open data map");

        for i in 0..100 {
            map.insert(i, i.to_string()).expect("Failed to insert");
        }

        storage.export(&file).expect("Failed to export storage");
    }

    let storage = RocksDB::open(temp_dir(), 0).expect("Failed to open storage");
    storage.import(&file).expect("Failed to import storage");

    let map = storage.open_map::<u32, String>(DataID::Test).expect("Failed to open data map");

    for i in 0..100 {
        assert_eq!(map.get(&i).expect("Failed to get").map(|v| v.to_string()), Some(i.to_string()));
    }
}
