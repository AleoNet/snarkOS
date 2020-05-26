use snarkos_dpc::base_dpc::{
    inner_circuit::InnerCircuit,
    instantiated::{Components, Predicate},
    parameters::CircuitParameters,
    record_payload::PaymentRecordPayload,
    BaseDPCComponents,
    DPC,
};
use snarkos_errors::dpc::DPCError;
use snarkos_models::{algorithms::CRH, parameters::Parameter};
use snarkos_objects::Account;
use snarkos_parameters::{GenesisAccount, GenesisPredicateVKBytes, SerialNumberNonceCRHParameters};
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

pub fn setup<C: BaseDPCComponents>() -> Result<(Vec<u8>, Vec<u8>), DPCError> {
    let rng = &mut thread_rng();

    let circuit_parameters = CircuitParameters::<C>::load()?;
    let serial_number_nonce_crh: C::SerialNumberNonceCRH =
        From::from(FromBytes::read(&SerialNumberNonceCRHParameters::load_bytes()[..])?);
    let serial_number_nonce_input: [u8; 32] = rng.gen();

    let genesis_serial_number_nonce = serial_number_nonce_crh.hash(&serial_number_nonce_input)?;
    let genesis_serial_number_nonce = to_bytes![genesis_serial_number_nonce]?;

    let genesis_predicate_vk_bytes = &GenesisPredicateVKBytes::load_bytes();
    let genesis_account: Account<C> = FromBytes::read(&GenesisAccount::load_bytes()[..])?;

    let genesis_record = DPC::<C>::generate_record(
        &circuit_parameters,
        &genesis_serial_number_nonce,
        &genesis_account.public_key,
        true, // The inital record should be dummy
        &PaymentRecordPayload::default(),
        &Predicate::new(genesis_predicate_vk_bytes.to_vec().clone()),
        &Predicate::new(genesis_predicate_vk_bytes.to_vec().clone()),
        rng,
    )?;

    let (genesis_record_serial_number, _) =
        DPC::generate_sn(&circuit_parameters, &genesis_record, &genesis_account.private_key).unwrap();

    let genesis_record_commitment = to_bytes![genesis_record.commitment()]?;
    let genesis_record_serial_number = to_bytes![genesis_sn]?;

    println!(
        "genesis_record_commitment\n\tsize - {}",
        genesis_record_commitment.len()
    );
    println!(
        "genesis_record_serial_number\n\tsize - {}",
        genesis_record_serial_number.len()
    );

    Ok((genesis_record_commitment, genesis_record_serial_number))
}

pub fn store(path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let (genesis_record_commitment, genesis_record_serial_number) = setup::<Components>().unwrap();
    store(&PathBuf::from("record_commitment.genesis"), &genesis_record_commitment).unwrap();
    store(
        &PathBuf::from("record_serial_number.genesis"),
        &genesis_record_serial_number,
    )
    .unwrap();
}
