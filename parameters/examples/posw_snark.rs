use snarkos_curves::bls12_377::Bls12_377;
use snarkos_errors::dpc::DPCError;
use snarkos_models::algorithms::SNARK;
use snarkos_posw::{Posw, GM17};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup() -> Result<(Vec<u8>, Vec<u8>), DPCError> {
    let rng = &mut thread_rng();

    let posw_snark = Posw::setup(rng).expect("could not setup params");
    let posw_snark_pk = to_bytes![posw_snark.pk.expect("posw_snark_pk should be populated")]?;
    let posw_snark_vk = <GM17<Bls12_377> as SNARK>::VerificationParameters::from(posw_snark.vk);
    let posw_snark_vk = to_bytes![posw_snark_vk]?;

    println!("posw_snark_pk.params\n\tsize - {}", posw_snark_pk.len());
    println!("posw_snark_vk.params\n\tsize - {}", posw_snark_vk.len());
    Ok((posw_snark_pk, posw_snark_vk))
}

pub fn main() {
    let (posw_snark_pk, posw_snark_vk) = setup().unwrap();
    store(
        &PathBuf::from("posw_snark_pk.params"),
        &PathBuf::from("posw_snark_pk.checksum"),
        &posw_snark_pk,
    )
    .unwrap();
    store(
        &PathBuf::from("posw_snark_vk.params"),
        &PathBuf::from("posw_snark_vk.checksum"),
        &posw_snark_vk,
    )
    .unwrap();
}
