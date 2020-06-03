use snarkos_algorithms::crh::sha256::sha256;
use snarkos_dpc::base_dpc::{instantiated::Components, BaseDPCComponents};
use snarkos_errors::algorithms::MerkleError;
use snarkos_models::algorithms::MerkleParameters;
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use hex;
use rand::thread_rng;
use std::{
    fs::{self, File},
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup<C: BaseDPCComponents>() -> Result<Vec<u8>, MerkleError> {
    let rng = &mut thread_rng();

    let ledger_merkle_tree_parameters = <C::MerkleParameters as MerkleParameters>::setup(rng);
    let ledger_merkle_tree_parameters_bytes = to_bytes![ledger_merkle_tree_parameters.parameters()]?;

    let size = ledger_merkle_tree_parameters_bytes.len();
    println!("ledger_merkle_tree.params\n\tsize - {}", size);
    Ok(ledger_merkle_tree_parameters_bytes)
}

pub fn store(file_path: &PathBuf, checksum_path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
    // Save checksum to file
    fs::write(checksum_path, hex::encode(sha256(bytes)))?;

    // Save buffer to file
    let mut file = File::create(file_path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("ledger_merkle_tree.params");
    let sumname = PathBuf::from("ledger_merkle_tree.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
