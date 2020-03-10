use crate::BlockStorage;

use std::{path::PathBuf, sync::Arc};

pub const TEST_DB_PATH: &str = "../test_db";
pub const GENESIS_BLOCK: &str = "0000000000000000000000000000000000000000000000000000000000000000b3d9ad9de8e21b2b3a9ffb40bae6fefa852026e7fb2e279322cd7589a20ee35592ec145e00000000ffffffffff7f000030d901000101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04080000000100e1f505000000001976a914ef5392fc02643be8b98f6aaca5c1ffaab238916a88ac";

pub fn initialize_test_blockchain() -> (Arc<BlockStorage>, PathBuf) {
    let mut path = std::env::current_dir().unwrap();
    path.push(random_storage_path());

    BlockStorage::destroy_storage(path.clone()).unwrap();

    let blockchain = BlockStorage::open_at_path(path.clone(), GENESIS_BLOCK.into()).unwrap();

    (blockchain, path)
}

pub fn random_storage_path() -> String {
    let ptr = Box::into_raw(Box::new(123));
    format!("{}{}", TEST_DB_PATH, ptr as usize)
}

pub fn kill_storage_async(path: PathBuf) {
    BlockStorage::destroy_storage(path).unwrap();
}

pub fn kill_storage_sync(storage: Arc<BlockStorage>, path: PathBuf) {
    drop(storage);
    BlockStorage::destroy_storage(path).unwrap();
}
