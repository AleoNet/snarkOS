use snarkos_dpc::base_dpc::instantiated::Components;
use snarkos_errors::algorithms::CommitmentError;
use snarkos_models::{algorithms::CommitmentScheme, dpc::DPCComponents};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::{
    fs::File,
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, CommitmentError> {
    let rng = &mut thread_rng();
    let account_commitment = <C::AccountCommitment as CommitmentScheme>::setup(rng);
    let account_commitment_parameters = account_commitment.parameters();
    let account_commitment_parameters_bytes = to_bytes![account_commitment_parameters]?;

    let size = account_commitment_parameters_bytes.len();
    println!("account_commitment.params\n\tsize - {}", size);
    Ok(account_commitment_parameters_bytes)
}

pub fn store(path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("account_commitment.params");
    store(&filename, &bytes).unwrap();
}
