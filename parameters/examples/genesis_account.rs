use snarkos_dpc::base_dpc::{
    inner_circuit::InnerCircuit,
    instantiated::{Components, Predicate},
    parameters::CircuitParameters,
    record_payload::PaymentRecordPayload,
    BaseDPCComponents,
    DPC,
};
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    algorithms::{SignatureScheme, CRH},
    objects::account::AccountScheme,
    parameters::Parameter,
};
use snarkos_objects::account::Account;
use snarkos_parameters::{AccountCommitmentParameters, AccountSignatureParameters};
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

    let account_signature_parameters: C::AccountSignature =
        From::from(FromBytes::read(&AccountSignatureParameters::load_bytes()[..])?);
    let account_commitment_parameters: C::AccountCommitment =
        From::from(FromBytes::read(&AccountCommitmentParameters::load_bytes()[..])?);
    let genesis_metadata: [u8; 32] = rng.gen();

    let genesis_account = Account::<C>::new(
        account_signature_parameters,
        account_commitment_parameters,
        &genesis_metadata,
        rng,
    )?;
    let genesis_account = to_bytes![genesis_account]?;

    println!("genesis_account\n\tsize - {}", genesis_account.len());
    Ok(genesis_account)
}

pub fn store(path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let genesis_account = setup::<Components>().unwrap();
    store(&PathBuf::from("account.genesis"), &genesis_account).unwrap();
}
