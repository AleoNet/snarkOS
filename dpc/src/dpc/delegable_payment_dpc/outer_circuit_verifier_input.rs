use crate::dpc::delegable_payment_dpc::{parameters::CommCRHSigPublicParameters, DelegablePaymentDPCComponents};
use snarkos_errors::{curves::ConstraintFieldError, gadgets::SynthesisError};
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    curves::to_field_vec::ToConstraintField,
};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DelegablePaymentDPCComponents"))]
pub struct OuterCircuitVerifierInput<C: DelegablePaymentDPCComponents> {
    pub comm_crh_sig_parameters: CommCRHSigPublicParameters<C>,
    pub predicate_comm: <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    pub local_data_comm: <C::LocalDataCommitment as CommitmentScheme>::Output,
}

impl<C: DelegablePaymentDPCComponents> ToConstraintField<C::OuterField> for OuterCircuitVerifierInput<C>
where
    <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::OuterField>,
    <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output: ToConstraintField<C::OuterField>,

    <C::PredicateVerificationKeyHash as CRH>::Parameters: ToConstraintField<C::OuterField>,

    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::LocalDataCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
    <C::ValueComm as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
{
    fn to_field_elements(&self) -> Result<Vec<C::OuterField>, ConstraintFieldError> {
        let mut v = Vec::new();

        v.extend_from_slice(
            &self
                .comm_crh_sig_parameters
                .pred_vk_comm_pp
                .parameters()
                .to_field_elements()?,
        );
        v.extend_from_slice(
            &self
                .comm_crh_sig_parameters
                .pred_vk_crh_pp
                .parameters()
                .to_field_elements()?,
        );

        let local_data_comm_pp_fe = ToConstraintField::<C::InnerField>::to_field_elements(
            self.comm_crh_sig_parameters.local_data_comm_pp.parameters(),
        )
        .map_err(|_| SynthesisError::AssignmentMissing)?;

        let local_data_comm_fe = ToConstraintField::<C::InnerField>::to_field_elements(&self.local_data_comm)
            .map_err(|_| SynthesisError::AssignmentMissing)?;

        let value_comm_pp_fe = ToConstraintField::<C::InnerField>::to_field_elements(
            self.comm_crh_sig_parameters.value_comm_pp.parameters(),
        )
        .map_err(|_| SynthesisError::AssignmentMissing)?;

        // Then we convert these field elements into bytes
        let pred_input = [
            to_bytes![local_data_comm_pp_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
            to_bytes![local_data_comm_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
            to_bytes![value_comm_pp_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
        ];

        // Then we convert them into `C::ProofCheckF::Fr` elements.
        v.extend_from_slice(&ToConstraintField::<C::OuterField>::to_field_elements(
            pred_input[0].as_slice(),
        )?);
        v.extend_from_slice(&ToConstraintField::<C::OuterField>::to_field_elements(
            pred_input[1].as_slice(),
        )?);
        v.extend_from_slice(&ToConstraintField::<C::OuterField>::to_field_elements(
            pred_input[2].as_slice(),
        )?);

        v.extend_from_slice(&self.predicate_comm.to_field_elements()?);
        Ok(v)
    }
}
