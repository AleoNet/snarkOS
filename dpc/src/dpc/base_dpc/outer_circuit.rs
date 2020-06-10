use crate::{
    dpc::base_dpc::{
        outer_circuit_gadget::execute_outer_proof_gadget,
        parameters::CircuitParameters,
        predicate::PrivatePredicateInput,
        BaseDPCComponents,
    },
    Assignment,
};
use snarkos_algorithms::merkle_tree::MerkleTreeDigest;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, SNARK},
    curves::to_field_vec::ToConstraintField,
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem},
};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct OuterCircuit<C: BaseDPCComponents> {
    circuit_parameters: Option<CircuitParameters<C>>,

    // Inner snark verifier public inputs
    ledger_parameters: Option<C::MerkleParameters>,
    ledger_digest: Option<MerkleTreeDigest<C::MerkleParameters>>,
    old_serial_numbers: Option<Vec<<C::AccountSignature as SignatureScheme>::PublicKey>>,
    new_commitments: Option<Vec<<C::RecordCommitment as CommitmentScheme>::Output>>,
    memo: Option<[u8; 32]>,
    value_balance: Option<i64>,

    // Inner snark verifier private inputs
    inner_snark_vk: Option<<C::InnerSNARK as SNARK>::VerificationParameters>,
    inner_snark_proof: Option<<C::InnerSNARK as SNARK>::Proof>,

    old_private_predicate_inputs: Option<Vec<PrivatePredicateInput<C>>>,
    new_private_predicate_inputs: Option<Vec<PrivatePredicateInput<C>>>,

    predicate_commitment: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output>,
    predicate_randomness: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness>,
    local_data_commitment: Option<<C::LocalDataCommitment as CommitmentScheme>::Output>,
}

impl<C: BaseDPCComponents> OuterCircuit<C> {
    pub fn blank(
        circuit_parameters: &CircuitParameters<C>,
        ledger_parameters: &C::MerkleParameters,
        inner_snark_vk: &<C::InnerSNARK as SNARK>::VerificationParameters,
        inner_snark_proof: &<C::InnerSNARK as SNARK>::Proof,
        predicate_nizk_vk_and_proof: &PrivatePredicateInput<C>,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        let ledger_digest = Some(MerkleTreeDigest::<C::MerkleParameters>::default());
        let old_serial_numbers = Some(vec![
            <C::AccountSignature as SignatureScheme>::PublicKey::default();
            num_input_records
        ]);
        let new_commitments = Some(vec![
            <C::RecordCommitment as CommitmentScheme>::Output::default();
            num_output_records
        ]);
        let memo = Some([0u8; 32]);
        let value_balance = Some(0);

        let old_private_predicate_inputs = Some(vec![predicate_nizk_vk_and_proof.clone(); num_input_records]);
        let new_private_predicate_inputs = Some(vec![predicate_nizk_vk_and_proof.clone(); num_output_records]);

        let predicate_commitment = Some(<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output::default());
        let predicate_randomness =
            Some(<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness::default());
        let local_data_comm = Some(<C::LocalDataCommitment as CommitmentScheme>::Output::default());

        Self {
            circuit_parameters: Some(circuit_parameters.clone()),

            ledger_parameters: Some(ledger_parameters.clone()),
            ledger_digest,
            old_serial_numbers,
            new_commitments,
            memo,
            value_balance,

            inner_snark_vk: Some(inner_snark_vk.clone()),
            inner_snark_proof: Some(inner_snark_proof.clone()),

            old_private_predicate_inputs,
            new_private_predicate_inputs,

            predicate_commitment,
            predicate_randomness,
            local_data_commitment: local_data_comm,
        }
    }

    pub fn new(
        circuit_parameters: &CircuitParameters<C>,

        // Inner snark public inputs
        ledger_parameters: &C::MerkleParameters,
        ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,
        old_serial_numbers: &Vec<<C::AccountSignature as SignatureScheme>::PublicKey>,
        new_commitments: &Vec<<C::RecordCommitment as CommitmentScheme>::Output>,
        memo: &[u8; 32],
        value_balance: i64,

        // Inner snark private inputs
        inner_snark_vk: &<C::InnerSNARK as SNARK>::VerificationParameters,
        inner_snark_proof: &<C::InnerSNARK as SNARK>::Proof,

        // Private predicate input = Verification key and input
        // Commitment contains commitment to hash of death predicate vk.
        old_private_predicate_inputs: &[PrivatePredicateInput<C>],

        // Private predicate input = Verification key and input
        // Commitment contains commitment to hash of birth predicate vk.
        new_private_predicate_inputs: &[PrivatePredicateInput<C>],

        predicate_commitment: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
        predicate_randomness: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,
        local_data_commitment: &<C::LocalDataCommitment as CommitmentScheme>::Output,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        assert_eq!(num_input_records, old_private_predicate_inputs.len());
        assert_eq!(num_output_records, new_private_predicate_inputs.len());

        Self {
            circuit_parameters: Some(circuit_parameters.clone()),

            ledger_parameters: Some(ledger_parameters.clone()),
            ledger_digest: Some(ledger_digest.clone()),
            old_serial_numbers: Some(old_serial_numbers.clone()),
            new_commitments: Some(new_commitments.clone()),
            memo: Some(memo.clone()),
            value_balance: Some(value_balance.clone()),

            inner_snark_vk: Some(inner_snark_vk.clone()),
            inner_snark_proof: Some(inner_snark_proof.clone()),

            old_private_predicate_inputs: Some(old_private_predicate_inputs.to_vec()),
            new_private_predicate_inputs: Some(new_private_predicate_inputs.to_vec()),

            predicate_commitment: Some(predicate_commitment.clone()),
            predicate_randomness: Some(predicate_randomness.clone()),
            local_data_commitment: Some(local_data_commitment.clone()),
        }
    }
}

impl<C: BaseDPCComponents> ConstraintSynthesizer<C::OuterField> for OuterCircuit<C>
where
    <C::LocalDataCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::ValueCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
    <C::ValueCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
{
    fn generate_constraints<CS: ConstraintSystem<C::OuterField>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_outer_proof_gadget::<C, CS>(
            cs,
            self.circuit_parameters.get()?,
            self.old_private_predicate_inputs.get()?.as_slice(),
            self.new_private_predicate_inputs.get()?.as_slice(),
            self.predicate_commitment.get()?,
            self.predicate_randomness.get()?,
            self.local_data_commitment.get()?,
        )?;
        Ok(())
    }
}
