use snarkos_algorithms::crh::sha256;
use snarkos_curves::bls12_377::Bls12_377;
use snarkos_errors::dpc::DPCError;
use snarkos_marlin::snark;
use snarkos_models::algorithms::SNARK;
use snarkos_posw::{Marlin, PoswMarlin};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup() -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), DPCError> {
    let rng = &mut thread_rng();

    let srs = snark::Marlin::<Bls12_377>::universal_setup(10000, 10000, 100000, rng).unwrap();
    let srs_bytes = to_bytes![srs]?;
    let posw_snark = PoswMarlin::index(srs).expect("could not setup params");

    let posw_snark_pk = to_bytes![posw_snark.pk.expect("posw_snark_pk should be populated")]?;
    let posw_snark_vk = <Marlin<Bls12_377> as SNARK>::VerificationParameters::from(posw_snark.vk);
    let posw_snark_vk = to_bytes![posw_snark_vk]?;

    println!("posw_snark_pk.params\n\tsize - {}", posw_snark_pk.len());
    println!("posw_snark_vk.params\n\tsize - {}", posw_snark_vk.len());
    println!("srs\n\tsize - {}", srs_bytes.len());
    Ok((posw_snark_pk, posw_snark_vk, srs_bytes))
}

fn versioned_filename(checksum: &str) -> String {
    match checksum.get(0..7) {
        Some(sum) => format!("posw_snark_pk-{}.params", sum),
        _ => format!("posw_snark_pk.params"),
    }
}

pub fn main() {
    let (posw_snark_pk, posw_snark_vk, _srs) = setup().unwrap();
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
