use snarkos_dpc::base_dpc::{instantiated::Components, BaseDPCComponents};
use snarkos_errors::algorithms::MerkleError;
use snarkos_models::algorithms::MerkleParameters;
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: BaseDPCComponents>() -> Result<Vec<u8>, MerkleError> {
    let rng = &mut thread_rng();

    let ledger_merkle_tree_parameters = <C::MerkleParameters as MerkleParameters>::setup(rng);
    let ledger_merkle_tree_parameters_bytes = to_bytes![ledger_merkle_tree_parameters.parameters()]?;

    let size = ledger_merkle_tree_parameters_bytes.len();
    println!("ledger_merkle_tree.params\n\tsize - {}", size);
    Ok(ledger_merkle_tree_parameters_bytes)
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("ledger_merkle_tree.params");
    let sumname = PathBuf::from("ledger_merkle_tree.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
