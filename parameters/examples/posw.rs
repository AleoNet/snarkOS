use snarkos_curves::bls12_377::Bls12_377;
use snarkos_errors::dpc::DPCError;
use snarkos_models::algorithms::SNARK;
use snarkos_posw::{Posw, Snark};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::{
    fs::File,
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup() -> Result<(Vec<u8>, Vec<u8>), DPCError> {
    let rng = &mut thread_rng();
    let posw = Posw::setup(rng).expect("could not setup params");

    let pk = to_bytes![posw.pk.expect("pk should be populated")]?;
    let vk = <Snark<Bls12_377> as SNARK>::VerificationParameters::from(posw.vk);
    let vk = to_bytes![vk]?;

    println!("posw_pk.params\n\tsize - {}", pk.len());
    println!("posw_vk.params\n\tsize - {}", vk.len());
    Ok((pk, vk))
}

pub fn store(path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let (pk, vk) = setup().unwrap();
    store(&PathBuf::from("posw_pk.params"), &pk).unwrap();
    store(&PathBuf::from("posw_vk.params"), &vk).unwrap();
}
