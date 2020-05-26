use snarkos_dpc::base_dpc::instantiated::Components;
use snarkos_errors::algorithms::SignatureError;
use snarkos_models::{algorithms::SignatureScheme, dpc::DPCComponents};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::{
    fs::File,
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, SignatureError> {
    let rng = &mut thread_rng();
    let account_signature = <C::AccountSignature as SignatureScheme>::setup(rng)?;
    let account_signature_parameters = account_signature.parameters();
    let account_signature_parameters_bytes = to_bytes![account_signature_parameters]?;

    let size = account_signature_parameters_bytes.len();
    println!("account_signature.params\n\tsize - {}", size);
    Ok(account_signature_parameters_bytes)
}

pub fn store(path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("account_signature.params");
    store(&filename, &bytes).unwrap();
}
