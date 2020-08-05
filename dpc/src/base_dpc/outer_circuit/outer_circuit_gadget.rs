use crate::base_dpc::{parameters::SystemParameters, program::PrivateProgramInput, BaseDPCComponents};
use snarkos_algorithms::merkle_tree::MerkleTreeDigest;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, EncryptionScheme, MerkleParameters, SignatureScheme, CRH, SNARK},
    curves::to_field_vec::ToConstraintField,
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget, SNARKVerifierGadget},
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            eq::EqGadget,
            uint::unsigned_integer::{UInt, UInt8},
            ToBytesGadget,
        },
    },
};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use itertools::Itertools;

fn field_element_to_bytes<C: BaseDPCComponents, CS: ConstraintSystem<C::OuterField>>(
    cs: &mut CS,
    field_elements: &Vec<C::InnerField>,
    name: &str,
) -> Result<Vec<Vec<UInt8>>, SynthesisError> {
    if field_elements.len() <= 1 {
        Ok(vec![UInt8::alloc_input_vec(
            cs.ns(|| format!("Allocate {}", name)),
            &to_bytes![field_elements].map_err(|_| SynthesisError::AssignmentMissing)?,
        )?])
    } else {
        let mut fe_bytes = vec![];
        for (index, field_element) in field_elements.iter().enumerate() {
            fe_bytes.push(UInt8::alloc_input_vec(
                cs.ns(|| format!("Allocate {} - index {} ", name, index)),
                &to_bytes![field_element].map_err(|_| SynthesisError::AssignmentMissing)?,
            )?);
        }
        Ok(fe_bytes)
    }
}

pub fn execute_outer_proof_gadget<C: BaseDPCComponents, CS: ConstraintSystem<C::OuterField>>(
    cs: &mut CS,
    // Parameters
    system_parameters: &SystemParameters<C>,

    // Inner snark verifier public inputs
    ledger_parameters: &C::MerkleParameters,
    ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,
    old_serial_numbers: &Vec<<C::AccountSignature as SignatureScheme>::PublicKey>,
    new_commitments: &Vec<<C::RecordCommitment as CommitmentScheme>::Output>,
    new_encrypted_record_hashes: &Vec<<C::EncryptedRecordCRH as CRH>::Output>,
    memo: &[u8; 32],
    value_balance: i64,
    network_id: u8,

    // Inner snark verifier private inputs (verification key and proof)
    inner_snark_vk: &<C::InnerSNARK as SNARK>::VerificationParameters,
    inner_snark_proof: &<C::InnerSNARK as SNARK>::Proof,

    // Old record death program verification keys and proofs
    old_death_program_verification_inputs: &[PrivateProgramInput<C::ProgramSNARK>],

    // New record birth program verification keys and proofs
    new_birth_program_verification_inputs: &[PrivateProgramInput<C::ProgramSNARK>],

    // Rest
    program_commitment: &<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output,
    program_randomness: &<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness,
    local_data_root: &<C::LocalDataCRH as CRH>::Output,
) -> Result<(), SynthesisError>
where
    <C::AccountCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::AccountCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::AccountEncryption as EncryptionScheme>::Parameters: ToConstraintField<C::InnerField>,

    <C::AccountSignature as SignatureScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::AccountSignature as SignatureScheme>::PublicKey: ToConstraintField<C::InnerField>,

    <C::RecordCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::RecordCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::EncryptedRecordCRH as CRH>::Parameters: ToConstraintField<C::InnerField>,
    <C::EncryptedRecordCRH as CRH>::Output: ToConstraintField<C::InnerField>,

    <C::SerialNumberNonceCRH as CRH>::Parameters: ToConstraintField<C::InnerField>,

    <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::LocalDataCRH as CRH>::Parameters: ToConstraintField<C::InnerField>,
    <C::LocalDataCRH as CRH>::Output: ToConstraintField<C::InnerField>,

    <C::ValueCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,

    <<C::MerkleParameters as MerkleParameters>::H as CRH>::Parameters: ToConstraintField<C::InnerField>,
    MerkleTreeDigest<C::MerkleParameters>: ToConstraintField<C::InnerField>,
{
    // Declare public parameters.
    let (program_vk_commitment_parameters, program_vk_crh_parameters) = {
        let cs = &mut cs.ns(|| "Declare Comm and CRH parameters");

        let program_vk_commitment_parameters = <C::ProgramVerificationKeyCommitmentGadget as CommitmentGadget<
            _,
            C::OuterField,
        >>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare program_vk_commitment_parameters"),
            || Ok(system_parameters.program_verification_key_commitment.parameters()),
        )?;

        let program_vk_crh_parameters =
            <C::ProgramVerificationKeyHashGadget as CRHGadget<_, C::OuterField>>::ParametersGadget::alloc_input(
                &mut cs.ns(|| "Declare program_vk_crh_parameters"),
                || Ok(system_parameters.program_verification_key_hash.parameters()),
            )?;

        (program_vk_commitment_parameters, program_vk_crh_parameters)
    };

    // ************************************************************************
    // Construct the InnerSNARK input
    // ************************************************************************

    // Declare inner snark verifier inputs as `CoreCheckF` field elements

    let account_commitment_parameters_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(system_parameters.account_commitment.parameters())
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let account_encryption_parameters_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(system_parameters.account_encryption.parameters())
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let account_signature_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(system_parameters.account_signature.parameters())
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let record_commitment_parameters_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(system_parameters.record_commitment.parameters())
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let encrypted_record_crh_parameters_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(system_parameters.encrypted_record_crh.parameters())
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let program_vk_commitment_parameters_fe = ToConstraintField::<C::InnerField>::to_field_elements(
        system_parameters.program_verification_key_commitment.parameters(),
    )
    .map_err(|_| SynthesisError::AssignmentMissing)?;

    let local_data_crh_parameters_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(system_parameters.local_data_crh.parameters())
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let serial_number_nonce_crh_parameters_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(system_parameters.serial_number_nonce.parameters())
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let value_commitment_parameters_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(system_parameters.value_commitment.parameters())
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let ledger_parameters_fe = ToConstraintField::<C::InnerField>::to_field_elements(ledger_parameters.parameters())
        .map_err(|_| SynthesisError::AssignmentMissing)?;

    let ledger_digest_fe = ToConstraintField::<C::InnerField>::to_field_elements(ledger_digest)
        .map_err(|_| SynthesisError::AssignmentMissing)?;

    let mut serial_numbers_fe = vec![];
    for sn in old_serial_numbers {
        let serial_number_fe =
            ToConstraintField::<C::InnerField>::to_field_elements(sn).map_err(|_| SynthesisError::AssignmentMissing)?;

        serial_numbers_fe.push(serial_number_fe);
    }

    let mut commitments_fe = vec![];
    for cm in new_commitments {
        let commitment_fe =
            ToConstraintField::<C::InnerField>::to_field_elements(cm).map_err(|_| SynthesisError::AssignmentMissing)?;

        commitments_fe.push(commitment_fe);
    }

    let mut encrypted_record_hashes_fe = vec![];
    for encrypted_record_hash in new_encrypted_record_hashes {
        let encrypted_record_hash_fe = ToConstraintField::<C::InnerField>::to_field_elements(encrypted_record_hash)
            .map_err(|_| SynthesisError::AssignmentMissing)?;

        encrypted_record_hashes_fe.push(encrypted_record_hash_fe);
    }

    let program_commitment_fe = ToConstraintField::<C::InnerField>::to_field_elements(program_commitment)
        .map_err(|_| SynthesisError::AssignmentMissing)?;

    let memo_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(memo).map_err(|_| SynthesisError::AssignmentMissing)?;

    let local_data_root_fe = ToConstraintField::<C::InnerField>::to_field_elements(local_data_root)
        .map_err(|_| SynthesisError::AssignmentMissing)?;

    let value_balance_as_u64 = value_balance.abs() as u64;

    let value_balance_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(&value_balance_as_u64.to_le_bytes()[..])
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let is_negative_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(&[value_balance.is_negative() as u8][..])
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let network_id_fe = ToConstraintField::<C::InnerField>::to_field_elements(&[network_id][..])
        .map_err(|_| SynthesisError::AssignmentMissing)?;

    // Allocate field element bytes

    let account_commitment_fe_bytes =
        field_element_to_bytes::<C, _>(cs, &account_commitment_parameters_fe, "account commitment pp")?;

    let account_encryption_fe_bytes =
        field_element_to_bytes::<C, _>(cs, &account_encryption_parameters_fe, "account encryption pp")?;

    let account_signature_fe_bytes = field_element_to_bytes::<C, _>(cs, &account_signature_fe, "account signature pp")?;
    let record_commitment_parameters_fe_bytes =
        field_element_to_bytes::<C, _>(cs, &record_commitment_parameters_fe, "record commitment pp")?;
    let encrypted_record_crh_parameters_fe_bytes =
        field_element_to_bytes::<C, _>(cs, &encrypted_record_crh_parameters_fe, "encrypted record crh pp")?;
    let program_vk_commitment_parameters_fe_bytes =
        field_element_to_bytes::<C, _>(cs, &program_vk_commitment_parameters_fe, "program vk commitment pp")?;
    let local_data_commitment_parameters_fe_bytes =
        field_element_to_bytes::<C, _>(cs, &local_data_crh_parameters_fe, "local data commitment pp")?;
    let serial_number_nonce_crh_parameters_fe_bytes =
        field_element_to_bytes::<C, _>(cs, &serial_number_nonce_crh_parameters_fe, "serial number nonce crh pp")?;
    let value_commitment_parameters_fe_bytes =
        field_element_to_bytes::<C, _>(cs, &value_commitment_parameters_fe, "value commitment pp")?;
    let ledger_parameters_fe_bytes = field_element_to_bytes::<C, _>(cs, &ledger_parameters_fe, "ledger pp")?;
    let ledger_digest_fe_bytes = field_element_to_bytes::<C, _>(cs, &ledger_digest_fe, "ledger digest")?;

    let mut serial_number_fe_bytes = vec![];
    for (index, sn_fe) in serial_numbers_fe.iter().enumerate() {
        serial_number_fe_bytes.extend(field_element_to_bytes::<C, _>(
            cs,
            sn_fe,
            &format!("Allocate serial number {:?}", index),
        )?);
    }

    let mut commitment_and_encrypted_record_hash_fe_bytes = vec![];
    for (index, (cm_fe, encrypted_record_hash_fe)) in
        commitments_fe.iter().zip_eq(&encrypted_record_hashes_fe).enumerate()
    {
        commitment_and_encrypted_record_hash_fe_bytes.extend(field_element_to_bytes::<C, _>(
            cs,
            cm_fe,
            &format!("Allocate record commitment {:?}", index),
        )?);

        commitment_and_encrypted_record_hash_fe_bytes.extend(field_element_to_bytes::<C, _>(
            cs,
            encrypted_record_hash_fe,
            &format!("Allocate encrypted record hash {:?}", index),
        )?);
    }

    let program_commitment_fe_bytes = field_element_to_bytes::<C, _>(cs, &program_commitment_fe, "program commitment")?;
    let memo_fe_bytes = field_element_to_bytes::<C, _>(cs, &memo_fe, "memo")?;
    let network_id_fe_bytes = field_element_to_bytes::<C, _>(cs, &network_id_fe, "network id")?;
    let local_data_root_fe_bytes = field_element_to_bytes::<C, _>(cs, &local_data_root_fe, "local data root")?;
    let value_balance_fe_bytes = field_element_to_bytes::<C, _>(cs, &value_balance_fe, "value balance")?;
    let is_negative_fe_bytes = field_element_to_bytes::<C, _>(cs, &is_negative_fe, "is_negative flag")?;

    // Construct inner snark input as bytes

    let mut inner_snark_input_bytes = vec![];
    inner_snark_input_bytes.extend(account_commitment_fe_bytes);
    inner_snark_input_bytes.extend(account_encryption_fe_bytes);
    inner_snark_input_bytes.extend(account_signature_fe_bytes);
    inner_snark_input_bytes.extend(record_commitment_parameters_fe_bytes);
    inner_snark_input_bytes.extend(encrypted_record_crh_parameters_fe_bytes);
    inner_snark_input_bytes.extend(program_vk_commitment_parameters_fe_bytes);
    inner_snark_input_bytes.extend(local_data_commitment_parameters_fe_bytes.clone());
    inner_snark_input_bytes.extend(serial_number_nonce_crh_parameters_fe_bytes);
    inner_snark_input_bytes.extend(value_commitment_parameters_fe_bytes);
    inner_snark_input_bytes.extend(ledger_parameters_fe_bytes);
    inner_snark_input_bytes.extend(ledger_digest_fe_bytes);
    inner_snark_input_bytes.extend(serial_number_fe_bytes);
    inner_snark_input_bytes.extend(commitment_and_encrypted_record_hash_fe_bytes);
    inner_snark_input_bytes.extend(program_commitment_fe_bytes);
    inner_snark_input_bytes.extend(memo_fe_bytes);
    inner_snark_input_bytes.extend(network_id_fe_bytes);
    inner_snark_input_bytes.extend(local_data_root_fe_bytes.clone());
    inner_snark_input_bytes.extend(value_balance_fe_bytes);
    inner_snark_input_bytes.extend(is_negative_fe_bytes);

    // Convert inner snark input bytes to bits

    let mut inner_snark_input_bits = vec![];
    for input_bytes in inner_snark_input_bytes {
        let input_bits = input_bytes
            .iter()
            .flat_map(|byte| byte.to_bits_le())
            .collect::<Vec<_>>();
        inner_snark_input_bits.push(input_bits);
    }

    // ************************************************************************
    // Verify the InnerSNARK proof
    // ************************************************************************

    let inner_snark_vk = <C::InnerSNARKGadget as SNARKVerifierGadget<_, _>>::VerificationKeyGadget::alloc(
        &mut cs.ns(|| "Allocate inner snark verification key"),
        || Ok(inner_snark_vk),
    )?;

    let inner_snark_proof = <C::InnerSNARKGadget as SNARKVerifierGadget<_, _>>::ProofGadget::alloc(
        &mut cs.ns(|| "Allocate inner snark proof"),
        || Ok(inner_snark_proof),
    )?;

    C::InnerSNARKGadget::check_verify(
        &mut cs.ns(|| "Check that proof is satisfied"),
        &inner_snark_vk,
        inner_snark_input_bits.iter().filter(|inp| !inp.is_empty()),
        &inner_snark_proof,
    )?;

    // ************************************************************************
    // Construct program input
    // ************************************************************************

    // Reuse inner snark verifier inputs

    let mut program_input_bytes = vec![];

    program_input_bytes.extend(local_data_commitment_parameters_fe_bytes);
    program_input_bytes.extend(local_data_root_fe_bytes);

    let mut program_input_bits = vec![];

    for input_bytes in program_input_bytes {
        let input_bits = input_bytes
            .iter()
            .flat_map(|byte| byte.to_bits_le())
            .collect::<Vec<_>>();
        program_input_bits.push(input_bits);
    }

    // ************************************************************************
    // ************************************************************************

    let mut old_death_program_ids = Vec::new();
    let mut new_birth_program_ids = Vec::new();
    for i in 0..C::NUM_INPUT_RECORDS {
        let cs = &mut cs.ns(|| format!("Check death program for input record {}", i));

        let death_program_proof = <C::ProgramSNARKGadget as SNARKVerifierGadget<_, _>>::ProofGadget::alloc(
            &mut cs.ns(|| "Allocate proof"),
            || Ok(&old_death_program_verification_inputs[i].proof),
        )?;

        let death_program_vk = <C::ProgramSNARKGadget as SNARKVerifierGadget<_, _>>::VerificationKeyGadget::alloc(
            &mut cs.ns(|| "Allocate verification key"),
            || Ok(&old_death_program_verification_inputs[i].verification_key),
        )?;

        let death_program_vk_bytes = death_program_vk.to_bytes(&mut cs.ns(|| "Convert death pred vk to bytes"))?;

        let claimed_death_program_id = C::ProgramVerificationKeyHashGadget::check_evaluation_gadget(
            &mut cs.ns(|| "Compute death program vk hash"),
            &program_vk_crh_parameters,
            &death_program_vk_bytes,
        )?;

        let claimed_death_program_id_bytes =
            claimed_death_program_id.to_bytes(&mut cs.ns(|| "Convert death_pred vk hash to bytes"))?;

        old_death_program_ids.push(claimed_death_program_id_bytes);

        let position = UInt8::constant(i as u8).to_bits_le();

        C::ProgramSNARKGadget::check_verify(
            &mut cs.ns(|| "Check that proof is satisfied"),
            &death_program_vk,
            ([position].iter())
                .chain(program_input_bits.iter())
                .filter(|inp| !inp.is_empty()),
            &death_program_proof,
        )?;
    }

    for j in 0..C::NUM_OUTPUT_RECORDS {
        let cs = &mut cs.ns(|| format!("Check birth program for output record {}", j));

        let birth_program_proof = <C::ProgramSNARKGadget as SNARKVerifierGadget<_, _>>::ProofGadget::alloc(
            &mut cs.ns(|| "Allocate proof"),
            || Ok(&new_birth_program_verification_inputs[j].proof),
        )?;

        let birth_program_vk = <C::ProgramSNARKGadget as SNARKVerifierGadget<_, _>>::VerificationKeyGadget::alloc(
            &mut cs.ns(|| "Allocate verification key"),
            || Ok(&new_birth_program_verification_inputs[j].verification_key),
        )?;

        let birth_program_vk_bytes = birth_program_vk.to_bytes(&mut cs.ns(|| "Convert birth pred vk to bytes"))?;

        let claimed_birth_program_id = C::ProgramVerificationKeyHashGadget::check_evaluation_gadget(
            &mut cs.ns(|| "Compute birth program vk hash"),
            &program_vk_crh_parameters,
            &birth_program_vk_bytes,
        )?;

        let claimed_birth_program_id_bytes =
            claimed_birth_program_id.to_bytes(&mut cs.ns(|| "Convert birth_pred vk hash to bytes"))?;

        new_birth_program_ids.push(claimed_birth_program_id_bytes);

        // TODO (raychu86) update this position to be (C::NUM_INPUT_RECORDS + j)
        let position = UInt8::constant(j as u8).to_bits_le();

        C::ProgramSNARKGadget::check_verify(
            &mut cs.ns(|| "Check that proof is satisfied"),
            &birth_program_vk,
            ([position].iter())
                .chain(program_input_bits.iter())
                .filter(|inp| !inp.is_empty()),
            &birth_program_proof,
        )?;
    }
    {
        let commitment_cs = &mut cs.ns(|| "Check that program commitment is well-formed");

        let mut input = Vec::new();
        for i in 0..C::NUM_INPUT_RECORDS {
            input.extend_from_slice(&old_death_program_ids[i]);
        }

        for j in 0..C::NUM_OUTPUT_RECORDS {
            input.extend_from_slice(&new_birth_program_ids[j]);
        }

        let given_commitment_randomness =
            <C::ProgramVerificationKeyCommitmentGadget as CommitmentGadget<_, C::OuterField>>::RandomnessGadget::alloc(
                &mut commitment_cs.ns(|| "Commitment randomness"),
                || Ok(program_randomness),
            )?;

        let given_commitment = <C::ProgramVerificationKeyCommitmentGadget as CommitmentGadget<_, C::OuterField>>::OutputGadget::alloc_input(
            &mut commitment_cs.ns(|| "Commitment output"),
            || Ok(program_commitment),
        )?;

        let candidate_commitment =
            <C::ProgramVerificationKeyCommitmentGadget as CommitmentGadget<_, C::OuterField>>::check_commitment_gadget(
                &mut commitment_cs.ns(|| "Compute commitment"),
                &program_vk_commitment_parameters,
                &input,
                &given_commitment_randomness,
            )?;

        candidate_commitment.enforce_equal(
            &mut commitment_cs.ns(|| "Check that declared and computed commitments are equal"),
            &given_commitment,
        )?;
    }
    Ok(())
}
