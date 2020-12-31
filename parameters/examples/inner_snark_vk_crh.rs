use snarkvm_dpc::base_dpc::instantiated::Components;
use snarkvm_errors::algorithms::CRHError;
use snarkvm_models::{algorithms::CRH, dpc::DPCComponents};
use snarkvm_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, CRHError> {
    let rng = &mut thread_rng();
    let inner_snark_vk_crh = <C::InnerSNARKVerificationKeyCRH as CRH>::setup(rng);
    let inner_snark_vk_crh_parameters = inner_snark_vk_crh.parameters();
    let inner_snark_vk_crh_parameters_bytes = to_bytes![inner_snark_vk_crh_parameters]?;

    let size = inner_snark_vk_crh_parameters_bytes.len();
    println!("inner_snark_vk_crh.params\n\tsize - {}", size);
    Ok(inner_snark_vk_crh_parameters_bytes)
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("inner_snark_vk_crh.params");
    let sumname = PathBuf::from("inner_snark_vk_crh.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
