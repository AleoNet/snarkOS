use crate::{
    constraints::{delegable_payment_dpc::execute_proof_check_gadget, Assignment},
    dpc::delegable_payment_dpc::{
        parameters::CommCRHSigPublicParameters,
        predicate::PrivatePredInput,
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
pub struct ProofCheckCircuit<C: DelegablePaymentDPCComponents> {
    comm_and_crh_parameters: Option<CommCRHSigPublicParameters<C>>,

    old_private_pred_inputs: Option<Vec<PrivatePredInput<C>>>,

    new_private_pred_inputs: Option<Vec<PrivatePredInput<C>>>,

    predicate_comm: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output>,
    predicate_rand: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness>,
    local_data_comm: Option<<C::LocalDataCommitment as CommitmentScheme>::Output>,
}

impl<C: DelegablePaymentDPCComponents> ProofCheckCircuit<C> {
    pub fn blank(
        comm_and_crh_parameters: &CommCRHSigPublicParameters<C>,
        predicate_nizk_vk_and_proof: &PrivatePredInput<C>,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        let old_private_pred_inputs = Some(vec![predicate_nizk_vk_and_proof.clone(); num_input_records]);
        let new_private_pred_inputs = Some(vec![predicate_nizk_vk_and_proof.clone(); num_output_records]);

        let predicate_comm = Some(<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output::default());
        let predicate_rand = Some(<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness::default());
        let local_data_comm = Some(<C::LocalDataCommitment as CommitmentScheme>::Output::default());

        Self {
            comm_and_crh_parameters: Some(comm_and_crh_parameters.clone()),

            old_private_pred_inputs,
            new_private_pred_inputs,

            predicate_comm,
            predicate_rand,
            local_data_comm,
        }
    }

    pub fn new(
        comm_and_crh_parameters: &CommCRHSigPublicParameters<C>,
        // Private pred input = Verification key and input
        // Commitment contains commitment to hash of death predicate vk.
        old_private_pred_inputs: &[PrivatePredInput<C>],

        // Private pred input = Verification key and input
        // Commitment contains commitment to hash of birth predicate vk.
        new_private_pred_inputs: &[PrivatePredInput<C>],

        predicate_comm: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
        predicate_rand: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,
        local_data_comm: &<C::LocalDataCommitment as CommitmentScheme>::Output,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        assert_eq!(num_input_records, old_private_pred_inputs.len());

        assert_eq!(num_output_records, new_private_pred_inputs.len());

        Self {
            comm_and_crh_parameters: Some(comm_and_crh_parameters.clone()),

            old_private_pred_inputs: Some(old_private_pred_inputs.to_vec()),

            new_private_pred_inputs: Some(new_private_pred_inputs.to_vec()),

            predicate_comm: Some(predicate_comm.clone()),
            predicate_rand: Some(predicate_rand.clone()),
            local_data_comm: Some(local_data_comm.clone()),
        }
    }
}

impl<C: DelegablePaymentDPCComponents> ConstraintSynthesizer<C::OuterField> for ProofCheckCircuit<C>
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
            self.old_private_pred_inputs.get()?.as_slice(),
            self.new_private_pred_inputs.get()?.as_slice(),
            self.predicate_comm.get()?,
            self.predicate_rand.get()?,
            self.local_data_comm.get()?,
        )?;
        Ok(())
    }
}
