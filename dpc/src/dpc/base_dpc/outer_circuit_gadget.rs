use crate::dpc::base_dpc::{parameters::CircuitParameters, predicate::PrivatePredicateInput, BaseDPCComponents};
use snarkos_models::{
    curves::to_field_vec::ToConstraintField,
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget, SNARKVerifierGadget},
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, eq::EqGadget, uint8::UInt8, ToBytesGadget},
    },
};
use snarkvm_errors::gadgets::SynthesisError;
use snarkvm_models::algorithms::{CommitmentScheme, CRH};
use snarkvm_utilities::{bytes::ToBytes, to_bytes};

pub fn execute_outer_proof_gadget<C: BaseDPCComponents, CS: ConstraintSystem<C::OuterField>>(
    cs: &mut CS,
    // Parameters
    circuit_parameters: &CircuitParameters<C>,

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
    <C::LocalDataCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::ValueCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
    <C::ValueCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
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
    // Construct predicate input
    // ************************************************************************

    // First we convert the input for the predicates into `CoreCheckF` field elements
    let local_data_commitment_parameters_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(circuit_parameters.local_data_commitment.parameters())
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let local_data_commitment_fe = ToConstraintField::<C::InnerField>::to_field_elements(local_data_commitment)
        .map_err(|_| SynthesisError::AssignmentMissing)?;

    let value_commitment_parameters_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(circuit_parameters.value_commitment.parameters())
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    // Then we convert these field elements into bytes
    let predicate_input = [
        to_bytes![local_data_commitment_parameters_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
        to_bytes![local_data_commitment_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
        to_bytes![value_commitment_parameters_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
    ];

    let predicate_input_bytes = [
        UInt8::alloc_input_vec(cs.ns(|| "Allocate local data pp "), &predicate_input[0])?,
        UInt8::alloc_input_vec(cs.ns(|| "Allocate local data comm"), &predicate_input[1])?,
        UInt8::alloc_input_vec(cs.ns(|| "Allocate value comm pp"), &predicate_input[2])?,
    ];

    let predicate_input_bits = [
        predicate_input_bytes[0]
            .iter()
            .flat_map(|byte| byte.into_bits_le())
            .collect::<Vec<_>>(),
        predicate_input_bytes[1]
            .iter()
            .flat_map(|byte| byte.into_bits_le())
            .collect::<Vec<_>>(),
        predicate_input_bytes[2]
            .iter()
            .flat_map(|byte| byte.into_bits_le())
            .collect::<Vec<_>>(),
    ];
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

        let position = UInt8::constant(i as u8).into_bits_le();

        // Convert the value commitment and value commitment randomness to bytes
        let value_commitment_randomness_fe = ToConstraintField::<C::InnerField>::to_field_elements(
            &to_bytes![old_death_predicate_verification_inputs[i].value_commitment_randomness]?[..],
        )
        .map_err(|_| SynthesisError::AssignmentMissing)?;

        let value_commitment_fe = ToConstraintField::<C::InnerField>::to_field_elements(
            &old_death_predicate_verification_inputs[i].value_commitment,
        )
        .map_err(|_| SynthesisError::AssignmentMissing)?;

        let value_commitment_inputs = [
            to_bytes![value_commitment_randomness_fe[0]].map_err(|_| SynthesisError::AssignmentMissing)?,
            to_bytes![value_commitment_randomness_fe[1]].map_err(|_| SynthesisError::AssignmentMissing)?,
            to_bytes![value_commitment_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
        ];

        let value_commitment_input_bytes = [
            UInt8::alloc_vec(
                cs.ns(|| format!("Allocate input value commitment randomness x {}", i)),
                &value_commitment_inputs[0],
            )?,
            UInt8::alloc_vec(
                cs.ns(|| format!("Allocate input value commitment randomness y {}", i)),
                &value_commitment_inputs[1],
            )?,
            UInt8::alloc_vec(
                cs.ns(|| format!("Allocate input value commitment {}", i)),
                &value_commitment_inputs[2],
            )?,
        ];

        let value_commitment_input_bits = [
            value_commitment_input_bytes[0]
                .iter()
                .flat_map(|byte| byte.into_bits_le())
                .collect::<Vec<_>>(),
            value_commitment_input_bytes[1]
                .iter()
                .flat_map(|byte| byte.into_bits_le())
                .collect::<Vec<_>>(),
            value_commitment_input_bytes[2]
                .iter()
                .flat_map(|byte| byte.into_bits_le())
                .collect::<Vec<_>>(),
        ];

        C::PredicateSNARKGadget::check_verify(
            &mut cs.ns(|| "Check that proof is satisfied"),
            &death_predicate_vk,
            ([position].iter())
                .chain(predicate_input_bits.iter())
                .filter(|inp| !inp.is_empty())
                .chain(value_commitment_input_bits.iter()),
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

        let position = UInt8::constant(j as u8).into_bits_le();

        // Convert the value commitment and value commitment randomness to bytes
        let value_commitment_randomness_fe = ToConstraintField::<C::InnerField>::to_field_elements(
            &to_bytes![new_birth_predicate_verification_inputs[j].value_commitment_randomness]?[..],
        )
        .map_err(|_| SynthesisError::AssignmentMissing)?;

        let value_commitment_fe = ToConstraintField::<C::InnerField>::to_field_elements(
            &new_birth_predicate_verification_inputs[j].value_commitment,
        )
        .map_err(|_| SynthesisError::AssignmentMissing)?;

        let value_commitment_inputs = [
            to_bytes![value_commitment_randomness_fe[0]].map_err(|_| SynthesisError::AssignmentMissing)?,
            to_bytes![value_commitment_randomness_fe[1]].map_err(|_| SynthesisError::AssignmentMissing)?,
            to_bytes![value_commitment_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
        ];

        let value_commitment_input_bytes = [
            UInt8::alloc_vec(
                cs.ns(|| format!("Allocate output value commitment randomness x {}", j)),
                &value_commitment_inputs[0],
            )?,
            UInt8::alloc_vec(
                cs.ns(|| format!("Allocate output value commitment randomness y {}", j)),
                &value_commitment_inputs[1],
            )?,
            UInt8::alloc_vec(
                cs.ns(|| format!("Allocate output value commitment {}", j)),
                &value_commitment_inputs[2],
            )?,
        ];

        let value_commitment_input_bits = [
            value_commitment_input_bytes[0]
                .iter()
                .flat_map(|byte| byte.into_bits_le())
                .collect::<Vec<_>>(),
            value_commitment_input_bytes[1]
                .iter()
                .flat_map(|byte| byte.into_bits_le())
                .collect::<Vec<_>>(),
            value_commitment_input_bytes[2]
                .iter()
                .flat_map(|byte| byte.into_bits_le())
                .collect::<Vec<_>>(),
        ];

        C::PredicateSNARKGadget::check_verify(
            &mut cs.ns(|| "Check that proof is satisfied"),
            &birth_predicate_vk,
            ([position].iter())
                .chain(predicate_input_bits.iter())
                .filter(|inp| !inp.is_empty())
                .chain(value_commitment_input_bits.iter()),
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
