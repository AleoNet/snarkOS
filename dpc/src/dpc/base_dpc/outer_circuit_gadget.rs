use crate::dpc::base_dpc::{parameters::CircuitParameters, predicate::PrivatePredicateInput, BaseDPCComponents};
use snarkos_algorithms::merkle_tree::MerkleTreeDigest;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, MerkleParameters, SignatureScheme, CRH, SNARK},
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

fn field_element_to_bytes<C: BaseDPCComponents, CS: ConstraintSystem<C::OuterField>>(
    cs: &mut CS,
    inputs: &mut Vec<Vec<UInt8>>,
    field_elements: &Vec<C::InnerField>,
    name: &str,
) -> Result<(), SynthesisError> {
    if field_elements.len() <= 1 {
        inputs.push(UInt8::alloc_input_vec(
            cs.ns(|| format!("Allocate {}", name)),
            &to_bytes![field_elements].map_err(|_| SynthesisError::AssignmentMissing)?,
        )?);
    } else {
        for (index, field_element) in field_elements.iter().enumerate() {
            inputs.push(UInt8::alloc_input_vec(
                cs.ns(|| format!("Allocate {} - index {} ", name, index)),
                &to_bytes![field_element].map_err(|_| SynthesisError::AssignmentMissing)?,
            )?);
        }
    }

    Ok(())
}

// TODO (raychu86) Remove duplicate parameter instantiation
pub fn execute_outer_proof_gadget<C: BaseDPCComponents, CS: ConstraintSystem<C::OuterField>>(
    cs: &mut CS,
    // Parameters
    circuit_parameters: &CircuitParameters<C>,

    // Inner snark verifier public inputs
    ledger_parameters: &C::MerkleParameters,
    ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,
    old_serial_numbers: &Vec<<C::AccountSignature as SignatureScheme>::PublicKey>,
    new_commitments: &Vec<<C::RecordCommitment as CommitmentScheme>::Output>,
    memo: &[u8; 32],
    value_balance: &i64,

    // Inner snark verifier private inputs (verification key and proof)
    inner_snark_vk: &<C::InnerSNARK as SNARK>::VerificationParameters,
    inner_snark_proof: &<C::InnerSNARK as SNARK>::Proof,

    // Old record death predicate verification keys and proofs
    old_death_predicate_verification_inputs: &[PrivatePredicateInput<C>],

    // New record birth predicate verification keys and proofs
    new_birth_predicate_verification_inputs: &[PrivatePredicateInput<C>],

    // Rest
    predicate_commitment: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    predicate_randomness: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,
    local_data_commitment: &<C::LocalDataCommitment as CommitmentScheme>::Output,
) -> Result<(), SynthesisError>
where
    <C::AccountCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::AccountCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::AccountSignature as SignatureScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::AccountSignature as SignatureScheme>::PublicKey: ToConstraintField<C::InnerField>,

    <C::RecordCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::RecordCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::SerialNumberNonceCRH as CRH>::Parameters: ToConstraintField<C::InnerField>,

    <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::LocalDataCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::ValueCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,

    <<C::MerkleParameters as MerkleParameters>::H as CRH>::Parameters: ToConstraintField<C::InnerField>,
    MerkleTreeDigest<C::MerkleParameters>: ToConstraintField<C::InnerField>,
{
    // Declare public parameters.
    let (predicate_vk_commitment_parameters, predicate_vk_crh_parameters) = {
        let cs = &mut cs.ns(|| "Declare Comm and CRH parameters");

        let predicate_vk_commitment_parameters = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<
            _,
            C::OuterField,
        >>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare predicate_vk_commitment_parameters"),
            || Ok(circuit_parameters.predicate_verification_key_commitment.parameters()),
        )?;

        let predicate_vk_crh_parameters =
            <C::PredicateVerificationKeyHashGadget as CRHGadget<_, C::OuterField>>::ParametersGadget::alloc_input(
                &mut cs.ns(|| "Declare predicate_vk_crh_parameters"),
                || Ok(circuit_parameters.predicate_verification_key_hash.parameters()),
            )?;

        (predicate_vk_commitment_parameters, predicate_vk_crh_parameters)
    };

    // ************************************************************************
    // Construct the InnerSNARK input
    // ************************************************************************
    let inner_snark_input_bits = {
        let cs = &mut cs.ns(|| "Declare inner snark verifier inputs");

        let account_commitment_parameters_fe =
            ToConstraintField::<C::InnerField>::to_field_elements(circuit_parameters.account_commitment.parameters())
                .map_err(|_| SynthesisError::AssignmentMissing)?;

        let account_signature_fe =
            ToConstraintField::<C::InnerField>::to_field_elements(circuit_parameters.account_signature.parameters())
                .map_err(|_| SynthesisError::AssignmentMissing)?;

        let record_commitment_parameters_fe =
            ToConstraintField::<C::InnerField>::to_field_elements(circuit_parameters.record_commitment.parameters())
                .map_err(|_| SynthesisError::AssignmentMissing)?;

        let predicate_vk_commitment_parameters_fe = ToConstraintField::<C::InnerField>::to_field_elements(
            circuit_parameters.predicate_verification_key_commitment.parameters(),
        )
        .map_err(|_| SynthesisError::AssignmentMissing)?;

        let local_data_commitment_parameters_fe = ToConstraintField::<C::InnerField>::to_field_elements(
            circuit_parameters.local_data_commitment.parameters(),
        )
        .map_err(|_| SynthesisError::AssignmentMissing)?;

        let serial_number_nonce_crh_parameters_fe =
            ToConstraintField::<C::InnerField>::to_field_elements(circuit_parameters.serial_number_nonce.parameters())
                .map_err(|_| SynthesisError::AssignmentMissing)?;

        let value_commitment_parameters_fe =
            ToConstraintField::<C::InnerField>::to_field_elements(circuit_parameters.value_commitment.parameters())
                .map_err(|_| SynthesisError::AssignmentMissing)?;

        let ledger_parameters_fe =
            ToConstraintField::<C::InnerField>::to_field_elements(ledger_parameters.parameters())
                .map_err(|_| SynthesisError::AssignmentMissing)?;

        let ledger_digest_fe = ToConstraintField::<C::InnerField>::to_field_elements(ledger_digest)
            .map_err(|_| SynthesisError::AssignmentMissing)?;

        let mut serial_numbers_fe = vec![];
        for sn in old_serial_numbers {
            let serial_number_fe = ToConstraintField::<C::InnerField>::to_field_elements(sn)
                .map_err(|_| SynthesisError::AssignmentMissing)?;

            serial_numbers_fe.push(serial_number_fe);
        }

        let mut commitments_fe = vec![];
        for cm in new_commitments {
            let commitment_fe = ToConstraintField::<C::InnerField>::to_field_elements(cm)
                .map_err(|_| SynthesisError::AssignmentMissing)?;

            commitments_fe.push(commitment_fe);
        }

        let predicate_commitment_fe = ToConstraintField::<C::InnerField>::to_field_elements(predicate_commitment)
            .map_err(|_| SynthesisError::AssignmentMissing)?;

        let memo_fe = ToConstraintField::<C::InnerField>::to_field_elements(memo)
            .map_err(|_| SynthesisError::AssignmentMissing)?;

        let local_data_commitment_fe = ToConstraintField::<C::InnerField>::to_field_elements(local_data_commitment)
            .map_err(|_| SynthesisError::AssignmentMissing)?;

        let value_balance_as_u64 = value_balance.abs() as u64;

        let value_balance_fe =
            ToConstraintField::<C::InnerField>::to_field_elements(&value_balance_as_u64.to_le_bytes()[..])
                .map_err(|_| SynthesisError::AssignmentMissing)?;

        let is_negative_fe =
            ToConstraintField::<C::InnerField>::to_field_elements(&[value_balance.is_negative() as u8][..])
                .map_err(|_| SynthesisError::AssignmentMissing)?;

        let mut inner_snark_input_bytes = vec![];

        field_element_to_bytes::<C, _>(
            cs,
            &mut inner_snark_input_bytes,
            &account_commitment_parameters_fe,
            "account commitment pp",
        )?;
        field_element_to_bytes::<C, _>(
            cs,
            &mut inner_snark_input_bytes,
            &account_signature_fe,
            "account signature pp",
        )?;
        field_element_to_bytes::<C, _>(
            cs,
            &mut inner_snark_input_bytes,
            &record_commitment_parameters_fe,
            "record commitment pp",
        )?;
        field_element_to_bytes::<C, _>(
            cs,
            &mut inner_snark_input_bytes,
            &predicate_vk_commitment_parameters_fe,
            "predicate vk commitment pp",
        )?;
        field_element_to_bytes::<C, _>(
            cs,
            &mut inner_snark_input_bytes,
            &local_data_commitment_parameters_fe,
            "local data commitment pp",
        )?;
        field_element_to_bytes::<C, _>(
            cs,
            &mut inner_snark_input_bytes,
            &serial_number_nonce_crh_parameters_fe,
            "serial number nonce crh pp",
        )?;
        field_element_to_bytes::<C, _>(
            cs,
            &mut inner_snark_input_bytes,
            &value_commitment_parameters_fe,
            "value commitment pp",
        )?;
        field_element_to_bytes::<C, _>(cs, &mut inner_snark_input_bytes, &ledger_parameters_fe, "ledger pp")?;
        field_element_to_bytes::<C, _>(cs, &mut inner_snark_input_bytes, &ledger_digest_fe, "ledger digest")?;

        for (index, sn_fe) in serial_numbers_fe.iter().enumerate() {
            field_element_to_bytes::<C, _>(
                cs,
                &mut inner_snark_input_bytes,
                sn_fe,
                &format!("Allocate serial number {:?}", index),
            )?;
        }

        for (index, cm_fe) in commitments_fe.iter().enumerate() {
            field_element_to_bytes::<C, _>(
                cs,
                &mut inner_snark_input_bytes,
                cm_fe,
                &format!("Allocate record commitment {:?}", index),
            )?;
        }

        field_element_to_bytes::<C, _>(
            cs,
            &mut inner_snark_input_bytes,
            &predicate_commitment_fe,
            "predicate commitment",
        )?;
        field_element_to_bytes::<C, _>(cs, &mut inner_snark_input_bytes, &memo_fe, "memo")?;
        field_element_to_bytes::<C, _>(
            cs,
            &mut inner_snark_input_bytes,
            &local_data_commitment_fe,
            "local data commitment",
        )?;
        field_element_to_bytes::<C, _>(cs, &mut inner_snark_input_bytes, &value_balance_fe, "value balance")?;
        field_element_to_bytes::<C, _>(cs, &mut inner_snark_input_bytes, &is_negative_fe, "is_negative flag")?;

        let mut inner_snark_input_bits = vec![];

        for input_bytes in inner_snark_input_bytes {
            let input_bits = input_bytes
                .iter()
                .flat_map(|byte| byte.to_bits_le())
                .collect::<Vec<_>>();
            inner_snark_input_bits.push(input_bits);
        }

        inner_snark_input_bits
    };

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
    // Construct predicate input
    // ************************************************************************

    // First we convert the input for the predicates into `CoreCheckF` field elements
    let local_data_commitment_parameters_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(circuit_parameters.local_data_commitment.parameters())
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let local_data_commitment_fe = ToConstraintField::<C::InnerField>::to_field_elements(local_data_commitment)
        .map_err(|_| SynthesisError::AssignmentMissing)?;

    // Then we convert these field elements into bytes
    let mut predicate_input_bytes = vec![];
    field_element_to_bytes::<C, _>(
        cs,
        &mut predicate_input_bytes,
        &local_data_commitment_parameters_fe,
        "local data pp",
    )?;
    field_element_to_bytes::<C, _>(
        cs,
        &mut predicate_input_bytes,
        &local_data_commitment_fe,
        "local data commitment",
    )?;

    let mut predicate_input_bits = vec![];

    for input_bytes in predicate_input_bytes {
        let input_bits = input_bytes
            .iter()
            .flat_map(|byte| byte.to_bits_le())
            .collect::<Vec<_>>();
        predicate_input_bits.push(input_bits);
    }

    // ************************************************************************
    // ************************************************************************

    let mut old_death_predicate_hashes = Vec::new();
    let mut new_birth_predicate_hashes = Vec::new();
    for i in 0..C::NUM_INPUT_RECORDS {
        let cs = &mut cs.ns(|| format!("Check death predicate for input record {}", i));

        let death_predicate_proof = <C::PredicateSNARKGadget as SNARKVerifierGadget<_, _>>::ProofGadget::alloc(
            &mut cs.ns(|| "Allocate proof"),
            || Ok(&old_death_predicate_verification_inputs[i].proof),
        )?;

        let death_predicate_vk = <C::PredicateSNARKGadget as SNARKVerifierGadget<_, _>>::VerificationKeyGadget::alloc(
            &mut cs.ns(|| "Allocate verification key"),
            || Ok(&old_death_predicate_verification_inputs[i].verification_key),
        )?;

        let death_predicate_vk_bytes = death_predicate_vk.to_bytes(&mut cs.ns(|| "Convert death pred vk to bytes"))?;

        let claimed_death_predicate_hash = C::PredicateVerificationKeyHashGadget::check_evaluation_gadget(
            &mut cs.ns(|| "Compute death predicate vk hash"),
            &predicate_vk_crh_parameters,
            &death_predicate_vk_bytes,
        )?;

        let claimed_death_predicate_hash_bytes =
            claimed_death_predicate_hash.to_bytes(&mut cs.ns(|| "Convert death_pred vk hash to bytes"))?;

        old_death_predicate_hashes.push(claimed_death_predicate_hash_bytes);

        let position = UInt8::constant(i as u8).to_bits_le();

        C::PredicateSNARKGadget::check_verify(
            &mut cs.ns(|| "Check that proof is satisfied"),
            &death_predicate_vk,
            ([position].iter())
                .chain(predicate_input_bits.iter())
                .filter(|inp| !inp.is_empty()),
            &death_predicate_proof,
        )?;
    }

    for j in 0..C::NUM_OUTPUT_RECORDS {
        let cs = &mut cs.ns(|| format!("Check birth predicate for output record {}", j));

        let birth_predicate_proof = <C::PredicateSNARKGadget as SNARKVerifierGadget<_, _>>::ProofGadget::alloc(
            &mut cs.ns(|| "Allocate proof"),
            || Ok(&new_birth_predicate_verification_inputs[j].proof),
        )?;

        let birth_predicate_vk = <C::PredicateSNARKGadget as SNARKVerifierGadget<_, _>>::VerificationKeyGadget::alloc(
            &mut cs.ns(|| "Allocate verification key"),
            || Ok(&new_birth_predicate_verification_inputs[j].verification_key),
        )?;

        let birth_predicate_vk_bytes = birth_predicate_vk.to_bytes(&mut cs.ns(|| "Convert birth pred vk to bytes"))?;

        let claimed_birth_predicate_hash = C::PredicateVerificationKeyHashGadget::check_evaluation_gadget(
            &mut cs.ns(|| "Compute birth predicate vk hash"),
            &predicate_vk_crh_parameters,
            &birth_predicate_vk_bytes,
        )?;

        let claimed_birth_predicate_hash_bytes =
            claimed_birth_predicate_hash.to_bytes(&mut cs.ns(|| "Convert birth_pred vk hash to bytes"))?;

        new_birth_predicate_hashes.push(claimed_birth_predicate_hash_bytes);

        let position = UInt8::constant(j as u8).to_bits_le();

        C::PredicateSNARKGadget::check_verify(
            &mut cs.ns(|| "Check that proof is satisfied"),
            &birth_predicate_vk,
            ([position].iter())
                .chain(predicate_input_bits.iter())
                .filter(|inp| !inp.is_empty()),
            &birth_predicate_proof,
        )?;
    }
    {
        let commitment_cs = &mut cs.ns(|| "Check that predicate commitment is well-formed");

        let mut input = Vec::new();
        for i in 0..C::NUM_INPUT_RECORDS {
            input.extend_from_slice(&old_death_predicate_hashes[i]);
        }

        for j in 0..C::NUM_OUTPUT_RECORDS {
            input.extend_from_slice(&new_birth_predicate_hashes[j]);
        }

        let given_commitment_randomness = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<
            _,
            C::OuterField,
        >>::RandomnessGadget::alloc(
            &mut commitment_cs.ns(|| "Commitment randomness"),
            || Ok(predicate_randomness),
        )?;

        let given_commitment = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<_, C::OuterField>>::OutputGadget::alloc_input(
            &mut commitment_cs.ns(|| "Commitment output"),
            || Ok(predicate_commitment),
        )?;

        let candidate_commitment = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<
            _,
            C::OuterField,
        >>::check_commitment_gadget(
            &mut commitment_cs.ns(|| "Compute commitment"),
            &predicate_vk_commitment_parameters,
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
