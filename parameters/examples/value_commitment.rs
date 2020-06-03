use snarkos_algorithms::crh::sha256::sha256;
use snarkos_dpc::base_dpc::{instantiated::Components, BaseDPCComponents};
use snarkos_errors::algorithms::CommitmentError;
use snarkos_models::algorithms::CommitmentScheme;
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use hex;
use rand::thread_rng;
use std::{
    fs::{self, File},
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup<C: BaseDPCComponents>() -> Result<Vec<u8>, CommitmentError> {
    let rng = &mut thread_rng();
    let value_commitment = <C::ValueCommitment as CommitmentScheme>::setup(rng);
    let value_commitment_parameters = value_commitment.parameters();
    let value_commitment_parameters_bytes = to_bytes![value_commitment_parameters]?;

    let size = value_commitment_parameters_bytes.len();
    println!("value_commitment.params\n\tsize - {}", size);
    Ok(value_commitment_parameters_bytes)
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
    let filename = PathBuf::from("value_commitment.params");
    let sumname = PathBuf::from("value_commitment.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
