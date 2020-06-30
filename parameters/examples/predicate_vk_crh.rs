use snarkos_dpc::base_dpc::instantiated::Components;
use snarkos_errors::algorithms::CRHError;
use snarkos_models::{algorithms::CRH, dpc::DPCComponents};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, CRHError> {
    let rng = &mut thread_rng();
    let predicate_vk_crh = <C::PredicateVerificationKeyHash as CRH>::setup(rng);
    let predicate_vk_crh_parameters = predicate_vk_crh.parameters();
    let predicate_vk_crh_parameters_bytes = to_bytes![predicate_vk_crh_parameters]?;

    let size = predicate_vk_crh_parameters_bytes.len();
    println!("predicate_vk_crh.params\n\tsize - {}", size);
    Ok(predicate_vk_crh_parameters_bytes)
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("predicate_vk_crh.params");
    let sumname = PathBuf::from("predicate_vk_crh.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
