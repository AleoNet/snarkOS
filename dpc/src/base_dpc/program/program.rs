use crate::base_dpc::BaseDPCComponents;
use snarkos_errors::curves::ConstraintFieldError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    curves::to_field_vec::ToConstraintField,
};

/// Program verification key and proof
/// Represented as bytes to be generic for any Program SNARK
pub struct PrivateProgramInput {
    pub verification_key: Vec<u8>,
    pub proof: Vec<u8>,
}

impl Clone for PrivateProgramInput {
    fn clone(&self) -> Self {
        Self {
            verification_key: self.verification_key.clone(),
            proof: self.proof.clone(),
        }
    }
}

pub struct ProgramLocalData<C: BaseDPCComponents> {
    pub local_data_commitment_parameters: <C::LocalDataCommitment as CommitmentScheme>::Parameters,
    // TODO (raychu86) add local_data_crh_parameters
    pub local_data_root: <C::LocalDataCRH as CRH>::Output,
    pub position: u8,
}

/// Convert each component to bytes and pack into field elements.
impl<C: BaseDPCComponents> ToConstraintField<C::InnerField> for ProgramLocalData<C>
where
    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::LocalDataCRH as CRH>::Output: ToConstraintField<C::InnerField>,
{
    fn to_field_elements(&self) -> Result<Vec<C::InnerField>, ConstraintFieldError> {
        let mut v = ToConstraintField::<C::InnerField>::to_field_elements(&[self.position][..])?;

        v.extend_from_slice(&self.local_data_commitment_parameters.to_field_elements()?);
        v.extend_from_slice(&self.local_data_root.to_field_elements()?);
        Ok(v)
    }
}
