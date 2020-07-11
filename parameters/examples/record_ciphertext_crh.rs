use snarkos_algorithms::crh::sha256::sha256;
use snarkos_dpc::base_dpc::instantiated::Components;
use snarkos_errors::algorithms::CRHError;
use snarkos_models::{algorithms::CRH, dpc::DPCComponents};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use hex;
use rand::thread_rng;
use std::{
    fs::{self, File},
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, CRHError> {
    let rng = &mut thread_rng();
    let record_ciphertext_crh = <C::RecordCiphertextCRH as CRH>::setup(rng);
    let record_ciphertext_crh_parameters = record_ciphertext_crh.parameters();
    let record_ciphertext_crh_parameters_bytes = to_bytes![record_ciphertext_crh_parameters]?;

    let size = record_ciphertext_crh_parameters_bytes.len();
    println!("record_ciphertext_crh.params\n\tsize - {}", size);
    Ok(record_ciphertext_crh_parameters_bytes)
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
    let filename = PathBuf::from("record_ciphertext_crh.params");
    let sumname = PathBuf::from("record_ciphertext_crh.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
