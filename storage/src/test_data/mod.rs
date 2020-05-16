use crate::BlockStorage;

use snarkos_algorithms::merkle_tree::MerkleParameters;
use snarkos_objects::{dpc::Transaction, ledger::Ledger};

use rand::{thread_rng, Rng};
use std::{path::PathBuf, sync::Arc};

pub fn random_storage_path() -> String {
    let random_path: usize = thread_rng().gen();
    format!("./test_db-{}", random_path)
}

// Initialize a test blockchain given genesis attributes
pub fn initialize_test_blockchain<T: Transaction, P: MerkleParameters>(
    parameters: P,
    genesis_cm: T::Commitment,
    genesis_sn: T::SerialNumber,
    genesis_memo: T::Memorandum,
    genesis_predicate_vk_bytes: Vec<u8>,
    genesis_account_bytes: Vec<u8>,
) -> BlockStorage<T, P> {
    let mut path = std::env::temp_dir();
    path.push(random_storage_path());

    BlockStorage::<T, P>::destroy_storage(path.clone()).unwrap();

    let storage = BlockStorage::<T, P>::new(
        &path,
        parameters,
        genesis_cm,
        genesis_sn,
        genesis_memo,
        genesis_predicate_vk_bytes,
        genesis_account_bytes,
    )
    .unwrap();

    storage
}

// Open a test blockchain from stored genesis attributes
pub fn test_blockchain<T: Transaction, P: MerkleParameters>() -> (Arc<BlockStorage<T, P>>, PathBuf) {
    let mut path = std::env::temp_dir();
    path.push(random_storage_path());

    BlockStorage::<T, P>::destroy_storage(path.clone()).unwrap();

    let storage = Arc::new(BlockStorage::<T, P>::open_at_path(path.clone()).unwrap());

    (storage, path)
}

pub fn kill_storage<T: Transaction, P: MerkleParameters>(ledger: BlockStorage<T, P>) {
    let path = ledger.storage.storage.path().to_owned();

    drop(ledger);
    BlockStorage::<T, P>::destroy_storage(path).unwrap();
}

pub fn kill_storage_async<T: Transaction, P: MerkleParameters>(path: PathBuf) {
    BlockStorage::<T, P>::destroy_storage(path).unwrap();
}

pub fn kill_storage_sync<T: Transaction, P: MerkleParameters>(ledger: Arc<BlockStorage<T, P>>) {
    let path = ledger.storage.storage.path().to_owned();

    drop(ledger);
    BlockStorage::<T, P>::destroy_storage(path).unwrap();
}
