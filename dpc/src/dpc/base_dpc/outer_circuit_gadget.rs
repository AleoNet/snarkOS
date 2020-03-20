use crate::dpc::base_dpc::{parameters::CircuitParameters, predicate::PrivatePredicateInput, BaseDPCComponents};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    curves::to_field_vec::ToConstraintField,
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget, SNARKVerifierGadget},
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, eq::EqGadget, uint8::UInt8, ToBytesGadget},
    },
};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

pub fn execute_outer_proof_gadget<C: BaseDPCComponents, CS: ConstraintSystem<C::OuterField>>(
    cs: &mut CS,
    // Parameters
    circuit_parameters: &CircuitParameters<C>,

    // Old record death predicate verif. keys and proofs
    old_death_pred_vk_and_pf: &[PrivatePredicateInput<C>],

    // New record birth predicate verif. keys and proofs
    new_birth_pred_vk_and_pf: &[PrivatePredicateInput<C>],

    // Rest
    predicate_comm: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    predicate_rand: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,

    local_data_comm: &<C::LocalDataCommitment as CommitmentScheme>::Output,
) -> Result<(), SynthesisError>
where
    <C::LocalDataCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::ValueCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
    <C::ValueCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
{
    // Declare public parameters.
    let (pred_vk_comm_pp, pred_vk_crh_pp) = {
        let cs = &mut cs.ns(|| "Declare Comm and CRH parameters");

        let pred_vk_comm_pp =
            <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<_, C::OuterField>>::ParametersGadget::alloc_input(
                &mut cs.ns(|| "Declare Pred Vk COMM parameters"),
                || Ok(circuit_parameters.predicate_verification_key_commitment_parameters.parameters()),
            )?;

        let pred_vk_crh_pp =
            <C::PredicateVerificationKeyHashGadget as CRHGadget<_, C::OuterField>>::ParametersGadget::alloc_input(
                &mut cs.ns(|| "Declare Pred Vk CRH parameters"),
                || {
                    Ok(circuit_parameters
                        .predicate_verification_key_hash_parameters
                        .parameters())
                },
            )?;

        (pred_vk_comm_pp, pred_vk_crh_pp)
    };

    // ************************************************************************
    // Construct predicate input
    // ************************************************************************

    // First we convert the input for the predicates into `CoreCheckF` field elements
    let local_data_comm_pp_fe = ToConstraintField::<C::InnerField>::to_field_elements(
        circuit_parameters.local_data_commitment_parameters.parameters(),
    )
    .map_err(|_| SynthesisError::AssignmentMissing)?;

    let local_data_comm_fe = ToConstraintField::<C::InnerField>::to_field_elements(local_data_comm)
        .map_err(|_| SynthesisError::AssignmentMissing)?;

    let value_comm_pp_fe = ToConstraintField::<C::InnerField>::to_field_elements(
        circuit_parameters.value_commitment_parameters.parameters(),
    )
    .map_err(|_| SynthesisError::AssignmentMissing)?;

    // Then we convert these field elements into bytes
    let pred_input = [
        to_bytes![local_data_comm_pp_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
        to_bytes![local_data_comm_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
        to_bytes![value_comm_pp_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
    ];

    let pred_input_bytes = [
        UInt8::alloc_input_vec(cs.ns(|| "Allocate local data pp "), &pred_input[0])?,
        UInt8::alloc_input_vec(cs.ns(|| "Allocate local data comm"), &pred_input[1])?,
        UInt8::alloc_input_vec(cs.ns(|| "Allocate value comm pp"), &pred_input[2])?,
    ];

    let pred_input_bits = [
        pred_input_bytes[0]
            .iter()
            .flat_map(|byte| byte.into_bits_le())
            .collect::<Vec<_>>(),
        pred_input_bytes[1]
            .iter()
            .flat_map(|byte| byte.into_bits_le())
            .collect::<Vec<_>>(),
        pred_input_bytes[2]
            .iter()
            .flat_map(|byte| byte.into_bits_le())
            .collect::<Vec<_>>(),
    ];
    // ************************************************************************
    // ************************************************************************

    let mut old_death_pred_hashes = Vec::new();
    let mut new_birth_pred_hashes = Vec::new();
    for i in 0..C::NUM_INPUT_RECORDS {
        let cs = &mut cs.ns(|| format!("Check death predicate for input record {}", i));

        let death_pred_proof = <C::PredicateSNARKGadget as SNARKVerifierGadget<_, _>>::ProofGadget::alloc(
            &mut cs.ns(|| "Allocate proof"),
            || Ok(&old_death_pred_vk_and_pf[i].proof),
        )?;

        let death_pred_vk = <C::PredicateSNARKGadget as SNARKVerifierGadget<_, _>>::VerificationKeyGadget::alloc(
            &mut cs.ns(|| "Allocate verification key"),
            || Ok(&old_death_pred_vk_and_pf[i].verification_key),
        )?;

        let death_pred_vk_bytes = death_pred_vk.to_bytes(&mut cs.ns(|| "Convert death pred vk to bytes"))?;

        let claimed_death_pred_hash = C::PredicateVerificationKeyHashGadget::check_evaluation_gadget(
            &mut cs.ns(|| "Compute death predicate vk hash"),
            &pred_vk_crh_pp,
            &death_pred_vk_bytes,
        )?;

        let claimed_death_pred_hash_bytes =
            claimed_death_pred_hash.to_bytes(&mut cs.ns(|| "Convert death_pred vk hash to bytes"))?;

        old_death_pred_hashes.push(claimed_death_pred_hash_bytes);

        let position = UInt8::constant(i as u8).into_bits_le();

        // Convert the value commitment and value commitment randomness to bytes
        let value_comm_randomness_fe = ToConstraintField::<C::InnerField>::to_field_elements(
            &to_bytes![old_death_pred_vk_and_pf[i].value_commitment_randomness]?[..],
        )
        .map_err(|_| SynthesisError::AssignmentMissing)?;

        let value_comm_fe =
            ToConstraintField::<C::InnerField>::to_field_elements(&old_death_pred_vk_and_pf[i].value_commitment)
                .map_err(|_| SynthesisError::AssignmentMissing)?;

        let value_comm_inputs = [
            to_bytes![value_comm_randomness_fe[0]].map_err(|_| SynthesisError::AssignmentMissing)?,
            to_bytes![value_comm_randomness_fe[1]].map_err(|_| SynthesisError::AssignmentMissing)?,
            to_bytes![value_comm_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
        ];

        let value_comm_input_bytes = [
            UInt8::alloc_vec(
                cs.ns(|| format!("Allocate input value commitment randomness x {}", i)),
                &value_comm_inputs[0],
            )?,
            UInt8::alloc_vec(
                cs.ns(|| format!("Allocate input value commitment randomness y {}", i)),
                &value_comm_inputs[1],
            )?,
            UInt8::alloc_vec(
                cs.ns(|| format!("Allocate input value commitment {}", i)),
                &value_comm_inputs[2],
            )?,
        ];

        let value_comm_input_bits = [
            value_comm_input_bytes[0]
                .iter()
                .flat_map(|byte| byte.into_bits_le())
                .collect::<Vec<_>>(),
            value_comm_input_bytes[1]
                .iter()
                .flat_map(|byte| byte.into_bits_le())
                .collect::<Vec<_>>(),
            value_comm_input_bytes[2]
                .iter()
                .flat_map(|byte| byte.into_bits_le())
                .collect::<Vec<_>>(),
        ];

        C::PredicateSNARKGadget::check_verify(
            &mut cs.ns(|| "Check that proof is satisfied"),
            &death_pred_vk,
            ([position].iter())
                .chain(pred_input_bits.iter())
                .filter(|inp| !inp.is_empty())
                .chain(value_comm_input_bits.iter()),
            &death_pred_proof,
        )?;
    }

    for j in 0..C::NUM_OUTPUT_RECORDS {
        let cs = &mut cs.ns(|| format!("Check birth predicate for output record {}", j));

        let birth_pred_proof = <C::PredicateSNARKGadget as SNARKVerifierGadget<_, _>>::ProofGadget::alloc(
            &mut cs.ns(|| "Allocate proof"),
            || Ok(&new_birth_pred_vk_and_pf[j].proof),
        )?;

        let birth_pred_vk = <C::PredicateSNARKGadget as SNARKVerifierGadget<_, _>>::VerificationKeyGadget::alloc(
            &mut cs.ns(|| "Allocate verification key"),
            || Ok(&new_birth_pred_vk_and_pf[j].verification_key),
        )?;

        let birth_pred_vk_bytes = birth_pred_vk.to_bytes(&mut cs.ns(|| "Convert birth pred vk to bytes"))?;

        let claimed_birth_pred_hash = C::PredicateVerificationKeyHashGadget::check_evaluation_gadget(
            &mut cs.ns(|| "Compute birth predicate vk hash"),
            &pred_vk_crh_pp,
            &birth_pred_vk_bytes,
        )?;

        let claimed_birth_pred_hash_bytes =
            claimed_birth_pred_hash.to_bytes(&mut cs.ns(|| "Convert birth_pred vk hash to bytes"))?;

        new_birth_pred_hashes.push(claimed_birth_pred_hash_bytes);

        let position = UInt8::constant(j as u8).into_bits_le();

        // Convert the value commitment and value commitment randomness to bytes
        let value_comm_randomness_fe = ToConstraintField::<C::InnerField>::to_field_elements(
            &to_bytes![new_birth_pred_vk_and_pf[j].value_commitment_randomness]?[..],
        )
        .map_err(|_| SynthesisError::AssignmentMissing)?;

        let value_comm_fe =
            ToConstraintField::<C::InnerField>::to_field_elements(&new_birth_pred_vk_and_pf[j].value_commitment)
                .map_err(|_| SynthesisError::AssignmentMissing)?;

        let value_comm_inputs = [
            to_bytes![value_comm_randomness_fe[0]].map_err(|_| SynthesisError::AssignmentMissing)?,
            to_bytes![value_comm_randomness_fe[1]].map_err(|_| SynthesisError::AssignmentMissing)?,
            to_bytes![value_comm_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
        ];

        let value_comm_input_bytes = [
            UInt8::alloc_vec(
                cs.ns(|| format!("Allocate output value commitment randomness x {}", j)),
                &value_comm_inputs[0],
            )?,
            UInt8::alloc_vec(
                cs.ns(|| format!("Allocate output value commitment randomness y {}", j)),
                &value_comm_inputs[1],
            )?,
            UInt8::alloc_vec(
                cs.ns(|| format!("Allocate output value commitment {}", j)),
                &value_comm_inputs[2],
            )?,
        ];

        let value_comm_input_bits = [
            value_comm_input_bytes[0]
                .iter()
                .flat_map(|byte| byte.into_bits_le())
                .collect::<Vec<_>>(),
            value_comm_input_bytes[1]
                .iter()
                .flat_map(|byte| byte.into_bits_le())
                .collect::<Vec<_>>(),
            value_comm_input_bytes[2]
                .iter()
                .flat_map(|byte| byte.into_bits_le())
                .collect::<Vec<_>>(),
        ];

        C::PredicateSNARKGadget::check_verify(
            &mut cs.ns(|| "Check that proof is satisfied"),
            &birth_pred_vk,
            ([position].iter())
                .chain(pred_input_bits.iter())
                .filter(|inp| !inp.is_empty())
                .chain(value_comm_input_bits.iter()),
            &birth_pred_proof,
        )?;
    }
    {
        let comm_cs = &mut cs.ns(|| "Check that predicate commitment is well-formed");

        let mut input = Vec::new();
        for i in 0..C::NUM_INPUT_RECORDS {
            input.extend_from_slice(&old_death_pred_hashes[i]);
        }

        for j in 0..C::NUM_OUTPUT_RECORDS {
            input.extend_from_slice(&new_birth_pred_hashes[j]);
        }

        let given_comm_rand = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<_, C::OuterField>>::RandomnessGadget::alloc(
            &mut comm_cs.ns(|| "Commitment randomness"),
            || Ok(predicate_rand),
        )?;

        let given_comm = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<_, C::OuterField>>::OutputGadget::alloc_input(
            &mut comm_cs.ns(|| "Commitment output"),
            || Ok(predicate_comm),
        )?;

        let candidate_commitment = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<
            _,
            C::OuterField,
        >>::check_commitment_gadget(
            &mut comm_cs.ns(|| "Compute commitment"),
            &pred_vk_comm_pp,
            &input,
            &given_comm_rand,
        )?;

        candidate_commitment.enforce_equal(
            &mut comm_cs.ns(|| "Check that declared and computed commitments are equal"),
            &given_comm,
        )?;
    }
    Ok(())
}
