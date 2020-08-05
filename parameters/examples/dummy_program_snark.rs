use snarkos_dpc::base_dpc::{instantiated::Components, parameters::SystemParameters, BaseDPCComponents, DPC};
use snarkos_errors::dpc::DPCError;
use snarkos_models::algorithms::SNARK;
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: BaseDPCComponents>() -> Result<(Vec<u8>, Vec<u8>), DPCError> {
    let rng = &mut thread_rng();
    let system_parameters = SystemParameters::<C>::load()?;

    let dummy_program_snark_parameters = DPC::<C>::generate_dummy_program_snark_parameters(&system_parameters, rng)?;
    let dummy_program_snark_pk = to_bytes![dummy_program_snark_parameters.proving_key]?;
    let dummy_program_snark_vk: <C::DummyProgramSNARK as SNARK>::VerificationParameters =
        dummy_program_snark_parameters.verification_key.into();
    let dummy_program_snark_vk = to_bytes![dummy_program_snark_vk]?;

    println!(
        "dummy_program_snark_pk.params\n\tsize - {}",
        dummy_program_snark_pk.len()
    );
    println!(
        "dummy_program_snark_vk.params\n\tsize - {}",
        dummy_program_snark_vk.len()
    );
    Ok((dummy_program_snark_pk, dummy_program_snark_vk))
}

pub fn main() {
    let (dummy_program_snark_pk, dummy_program_snark_vk) = setup::<Components>().unwrap();
    store(
        &PathBuf::from("dummy_program_snark_pk.params"),
        &PathBuf::from("dummy_program_snark_pk.checksum"),
        &dummy_program_snark_pk,
    )
    .unwrap();
    store(
        &PathBuf::from("dummy_program_snark_vk.params"),
        &PathBuf::from("dummy_program_snark_vk.checksum"),
        &dummy_program_snark_vk,
    )
    .unwrap();
}
