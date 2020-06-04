use snarkos_algorithms::crh::sha256::sha256;
use snarkos_curves::bls12_377::Bls12_377;
use snarkos_errors::dpc::DPCError;
use snarkos_models::algorithms::SNARK;
use snarkos_posw::{Posw, Snark};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::{
    fs::{self, File},
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup() -> Result<(Vec<u8>, Vec<u8>), DPCError> {
    let rng = &mut thread_rng();

    let posw_snark = Posw::setup(rng).expect("could not setup params");
    let posw_snark_pk = to_bytes![posw_snark.pk.expect("posw_snark_pk should be populated")]?;
    let posw_snark_vk = <Snark<Bls12_377> as SNARK>::VerificationParameters::from(posw_snark.vk);
    let posw_snark_vk = to_bytes![posw_snark_vk]?;

    println!("posw_snark_pk.params\n\tsize - {}", posw_snark_pk.len());
    println!("posw_snark_vk.params\n\tsize - {}", posw_snark_vk.len());
    Ok((posw_snark_pk, posw_snark_vk))
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
