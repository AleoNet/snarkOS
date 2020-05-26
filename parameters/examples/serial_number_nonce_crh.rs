use snarkos_dpc::base_dpc::instantiated::Components;
use snarkos_errors::algorithms::CRHError;
use snarkos_models::{algorithms::CRH, dpc::DPCComponents};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::{
    fs::File,
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, CRHError> {
    let rng = &mut thread_rng();
    let serial_number_nonce_crh = <C::SerialNumberNonceCRH as CRH>::setup(rng);
    let serial_number_nonce_crh_parameters = serial_number_nonce_crh.parameters();
    let serial_number_nonce_crh_parameters_bytes = to_bytes![serial_number_nonce_crh_parameters]?;

    let size = serial_number_nonce_crh_parameters_bytes.len();
    println!("serial_number_nonce_crh.params\n\tsize - {}", size);
    Ok(serial_number_nonce_crh_parameters_bytes)
}

pub fn store(path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("serial_number_nonce_crh.params");
    store(&filename, &bytes).unwrap();
}
