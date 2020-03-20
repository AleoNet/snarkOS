use crate::{
    constraints::{delegable_payment_dpc::execute_proof_check_gadget, Assignment},
    dpc::delegable_payment_dpc::{
        parameters::CommCRHSigPublicParameters,
        predicate::PrivatePredicateInput,
        DelegablePaymentDPCComponents,
    },
};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::CommitmentScheme,
    curves::to_field_vec::ToConstraintField,
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem},
};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DelegablePaymentDPCComponents"))]
pub struct OuterCircuit<C: DelegablePaymentDPCComponents> {
    comm_and_crh_parameters: Option<CommCRHSigPublicParameters<C>>,

    old_private_predicate_inputs: Option<Vec<PrivatePredicateInput<C>>>,
    new_private_predicate_inputs: Option<Vec<PrivatePredicateInput<C>>>,

    predicate_commitment: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output>,
    predicate_randomness: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness>,
    local_data_comm: Option<<C::LocalDataCommitment as CommitmentScheme>::Output>,
}

impl<C: DelegablePaymentDPCComponents> OuterCircuit<C> {
    pub fn blank(
        comm_and_crh_parameters: &CommCRHSigPublicParameters<C>,
        predicate_nizk_vk_and_proof: &PrivatePredicateInput<C>,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        let old_private_predicate_inputs = Some(vec![predicate_nizk_vk_and_proof.clone(); num_input_records]);
        let new_private_predicate_inputs = Some(vec![predicate_nizk_vk_and_proof.clone(); num_output_records]);

        let predicate_commitment = Some(<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output::default());
        let predicate_randomness =
            Some(<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness::default());
        let local_data_comm = Some(<C::LocalDataCommitment as CommitmentScheme>::Output::default());

        Self {
            comm_and_crh_parameters: Some(comm_and_crh_parameters.clone()),

            old_private_predicate_inputs,
            new_private_predicate_inputs,

            predicate_commitment,
            predicate_randomness,
            local_data_comm,
        }
    }

    pub fn new(
        comm_and_crh_parameters: &CommCRHSigPublicParameters<C>,
        // Private pred input = Verification key and input
        // Commitment contains commitment to hash of death predicate vk.
        old_private_predicate_inputs: &[PrivatePredicateInput<C>],

        // Private pred input = Verification key and input
        // Commitment contains commitment to hash of birth predicate vk.
        new_private_predicate_inputs: &[PrivatePredicateInput<C>],

        predicate_commitment: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
        predicate_randomness: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,
        local_data_comm: &<C::LocalDataCommitment as CommitmentScheme>::Output,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        assert_eq!(num_input_records, old_private_predicate_inputs.len());
        assert_eq!(num_output_records, new_private_predicate_inputs.len());

        Self {
            comm_and_crh_parameters: Some(comm_and_crh_parameters.clone()),
            old_private_predicate_inputs: Some(old_private_predicate_inputs.to_vec()),
            new_private_predicate_inputs: Some(new_private_predicate_inputs.to_vec()),
            predicate_commitment: Some(predicate_commitment.clone()),
            predicate_randomness: Some(predicate_randomness.clone()),
            local_data_comm: Some(local_data_comm.clone()),
        }
    }
}

impl<C: DelegablePaymentDPCComponents> ConstraintSynthesizer<C::OuterField> for OuterCircuit<C>
where
    <C::LocalDataCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::ValueComm as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
    <C::ValueComm as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
{
    fn generate_constraints<CS: ConstraintSystem<C::OuterField>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_proof_check_gadget::<C, CS>(
            cs,
            self.comm_and_crh_parameters.get()?,
            self.old_private_predicate_inputs.get()?.as_slice(),
            self.new_private_predicate_inputs.get()?.as_slice(),
            self.predicate_commitment.get()?,
            self.predicate_randomness.get()?,
            self.local_data_comm.get()?,
        )?;
        Ok(())
    }
}
