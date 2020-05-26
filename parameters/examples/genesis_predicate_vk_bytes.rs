use snarkos_dpc::base_dpc::{instantiated::Components, BaseDPCComponents};
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    algorithms::{CRH, SNARK},
    parameters::Parameter,
};
use snarkos_parameters::{PredicateSNARKVKParameters, PredicateVKCRHParameters};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use rand::{thread_rng, Rng};
use std::{
    fs::File,
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup<C: BaseDPCComponents>() -> Result<Vec<u8>, DPCError> {
    let rng = &mut thread_rng();

    let predicate_vk_crh: C::PredicateVerificationKeyHash =
        From::from(FromBytes::read(&PredicateVKCRHParameters::load_bytes()[..])?);
    let predicate_snark_vk: <C::PredicateSNARK as SNARK>::VerifcationParameters =
        From::from(FromBytes::read(&PredicateSNARKVKParameters::load_bytes()[..])?);

    let genesis_predicate_vk_bytes = predicate_vk_crh.hash(&to_bytes![predicate_snark_vk]?)?;
    let genesis_predicate_vk_bytes = to_bytes![genesis_predicate_vk]?;

    println!(
        "genesis_predicate_vk_bytes\n\tsize - {}",
        genesis_predicate_vk_bytes.len()
    );

    Ok(genesis_predicate_vk_bytes)
}

pub fn store(path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let genesis_predicate_vk_bytes = setup::<Components>().unwrap();
    store(
        &PathBuf::from("predicate_vk_bytes.genesis"),
        &genesis_predicate_vk_bytes,
    )
    .unwrap();
}
