use snarkos_dpc::base_dpc::instantiated::Components;
use snarkos_errors::algorithms::CommitmentError;
use snarkos_models::{algorithms::CommitmentScheme, dpc::DPCComponents};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, CommitmentError> {
    let rng = &mut thread_rng();
    let local_data_commitment = <C::LocalDataCommitment as CommitmentScheme>::setup(rng);
    let local_data_commitment_parameters = local_data_commitment.parameters();
    let local_data_commitment_parameters_bytes = to_bytes![local_data_commitment_parameters]?;

    let size = local_data_commitment_parameters_bytes.len();
    println!("local_data_commitment.params\n\tsize - {}", size);
    Ok(local_data_commitment_parameters_bytes)
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("local_data_commitment.params");
    let sumname = PathBuf::from("local_data_commitment.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
