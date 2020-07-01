use snarkos_dpc::base_dpc::instantiated::Components;
use snarkos_errors::algorithms::SignatureError;
use snarkos_models::{algorithms::SignatureScheme, dpc::DPCComponents};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use hex;
use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, SignatureError> {
    let rng = &mut thread_rng();
    let account_signature = <C::AccountSignature as SignatureScheme>::setup(rng)?;
    let account_signature_parameters = account_signature.parameters();
    let account_signature_parameters_bytes = to_bytes![account_signature_parameters]?;

    let size = account_signature_parameters_bytes.len();
    println!("account_signature.params\n\tsize - {}", size);
    Ok(account_signature_parameters_bytes)
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("account_signature.params");
    let sumname = PathBuf::from("account_signature.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
