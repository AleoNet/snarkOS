use snarkos_dpc::base_dpc::{
    instantiated::Components,
    outer_circuit::OuterCircuit,
    parameters::CircuitParameters,
    payment_circuit::PaymentCircuit,
    predicate::PrivatePredicateInput,
    BaseDPCComponents,
    DPC,
};
use snarkos_errors::dpc::DPCError;
use snarkos_models::algorithms::{CommitmentScheme, SNARK};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::{
    fs::File,
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup<C: BaseDPCComponents>() -> Result<(Vec<u8>, Vec<u8>), DPCError> {
    let rng = &mut thread_rng();
    let circuit_parameters = CircuitParameters::<C>::load()?;

    let predicate_snark_parameters = DPC::<C>::generate_predicate_snark_parameters(&circuit_parameters, rng)?;
    let predicate_snark_proof = C::PredicateSNARK::prove(
        &predicate_snark_parameters.proving_key,
        PaymentCircuit::blank(&circuit_parameters),
        rng,
    )?;
    let private_predicate_input = PrivatePredicateInput {
        verification_key: predicate_snark_parameters.verification_key,
        proof: predicate_snark_proof,
        value_commitment: <C::ValueCommitment as CommitmentScheme>::Output::default(),
        value_commitment_randomness: <C::ValueCommitment as CommitmentScheme>::Randomness::default(),
    };

    let outer_snark_parameters =
        C::OuterSNARK::setup(OuterCircuit::blank(&circuit_parameters, &private_predicate_input), rng)?;
    let outer_snark_pk = to_bytes![outer_snark_parameters.0]?;
    let outer_snark_vk: <C::OuterSNARK as SNARK>::VerificationParameters = outer_snark_parameters.1.into();
    let outer_snark_vk = to_bytes![outer_snark_vk]?;

    println!("outer_snark_pk.params\n\tsize - {}", outer_snark_pk.len());
    println!("outer_snark_vk.params\n\tsize - {}", outer_snark_vk.len());
    Ok((outer_snark_pk, outer_snark_vk))
}

pub fn store(path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let (outer_snark_pk, outer_snark_vk) = setup::<Components>().unwrap();
    store(&PathBuf::from("outer_snark_pk.params"), &outer_snark_pk).unwrap();
    store(&PathBuf::from("outer_snark_vk.params"), &outer_snark_vk).unwrap();
}
