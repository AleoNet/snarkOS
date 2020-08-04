use crate::dpc::base_dpc::BaseDPCComponents;
use snarkos_errors::curves::ConstraintFieldError;
use snarkos_models::{
    algorithms::{CommitmentScheme, SNARK},
    curves::to_field_vec::ToConstraintField,
    dpc::Program,
};

use std::marker::PhantomData;

pub struct PrivateProgramInput<S: SNARK> {
    pub verification_key: S::VerificationParameters,
    pub proof: S::Proof,
}

impl<S: SNARK> Clone for PrivateProgramInput<S> {
    fn clone(&self) -> Self {
        Self {
            verification_key: self.verification_key.clone(),
            proof: self.proof.clone(),
        }
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "S: SNARK"), Default(bound = "S: SNARK"))]
pub struct DPCProgram<S: SNARK> {
    #[derivative(Default(value = "vec![0u8; 48]"))]
    identity: Vec<u8>,
    _components: PhantomData<S>,
}

impl<S: SNARK> DPCProgram<S> {
    pub fn new(identity: Vec<u8>) -> Self {
        Self {
            identity,
            _components: PhantomData,
        }
    }
}

impl<S: SNARK> Program for DPCProgram<S> {
    type PrivateWitness = PrivateProgramInput<S>;
    type PublicInput = ();

    fn evaluate(&self, _p: &Self::PublicInput, _w: &Self::PrivateWitness) -> bool {
        unimplemented!()
    }

    fn into_compact_repr(&self) -> Vec<u8> {
        self.identity.clone()
    }
}

pub struct ProgramLocalData<C: BaseDPCComponents> {
    pub local_data_commitment_parameters: <C::LocalDataCommitment as CommitmentScheme>::Parameters,
    // TODO (raychu86) add local_data_crh_parameters
    pub local_data_root: <C::LocalDataCommitment as CommitmentScheme>::Output,
    pub position: u8,
}

/// Convert each component to bytes and pack into field elements.
impl<C: BaseDPCComponents> ToConstraintField<C::InnerField> for ProgramLocalData<C>
where
    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::LocalDataCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
{
    fn to_field_elements(&self) -> Result<Vec<C::InnerField>, ConstraintFieldError> {
        let mut v = ToConstraintField::<C::InnerField>::to_field_elements(&[self.position][..])?;

        v.extend_from_slice(&self.local_data_commitment_parameters.to_field_elements()?);
        v.extend_from_slice(&self.local_data_root.to_field_elements()?);
        Ok(v)
    }
}
