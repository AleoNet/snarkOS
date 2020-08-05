use crate::base_dpc::{program::ProgramCircuit, BaseDPCComponents, LocalData};
use snarkos_errors::{curves::ConstraintFieldError, dpc::DPCError};
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH, SNARK},
    curves::to_field_vec::ToConstraintField,
    dpc::{Program, Record},
};
use snarkos_utilities::{to_bytes, ToBytes};

use rand::Rng;
use std::marker::PhantomData;

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

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: BaseDPCComponents, S: SNARK"),
    Debug(bound = "C: BaseDPCComponents, S: SNARK"),
    PartialEq(bound = "C: BaseDPCComponents, S: SNARK"),
    Eq(bound = "C: BaseDPCComponents, S: SNARK")
)]
pub struct DPCProgram<C: BaseDPCComponents, S: SNARK> {
    #[derivative(Default(value = "vec![0u8; 48]"))]
    identity: Vec<u8>,
    _components: PhantomData<C>,
    _snark: PhantomData<S>,
}

impl<C: BaseDPCComponents, S: SNARK> DPCProgram<C, S> {
    pub fn new(identity: Vec<u8>) -> Self {
        Self {
            identity,
            _components: PhantomData,
            _snark: PhantomData,
        }
    }
}

impl<C: BaseDPCComponents, S: SNARK> Program for DPCProgram<C, S>
where
    S: SNARK<AssignedCircuit = ProgramCircuit<C>, VerifierInput = ProgramLocalData<C>>,
{
    type LocalData = LocalData<C>;
    type PrivateWitness = PrivateProgramInput;
    type ProvingParameters = S::ProvingParameters;
    type PublicInput = ();
    type VerificationParameters = S::VerificationParameters;

    fn execute<R: Rng>(
        &self,
        proving_key: &Self::ProvingParameters,
        verification_key: &Self::VerificationParameters,
        local_data: &Self::LocalData,
        position: u8,
        rng: &mut R,
    ) -> Result<Self::PrivateWitness, DPCError> {
        let mut position = position;
        let records = [&local_data.old_records[..], &local_data.new_records[..]].concat();
        assert!((position as usize) < records.len());

        let record = &records[position as usize];

        if (position as usize) < C::NUM_INPUT_RECORDS {
            assert_eq!(self.identity, record.death_program_id());
        } else {
            assert_eq!(self.identity, record.birth_program_id());

            // TODO (raychu86) Make this position absolute (remove this line)
            position -= C::NUM_INPUT_RECORDS as u8;
        }

        let circuit = ProgramCircuit::<C>::new(&local_data.system_parameters, &local_data.local_data_root, position);

        let proof = S::prove(proving_key, circuit, rng)?;

        {
            let program_snark_pvk: <S as SNARK>::PreparedVerificationParameters = verification_key.clone().into();

            let program_pub_input: ProgramLocalData<C> = ProgramLocalData {
                local_data_commitment_parameters: local_data
                    .system_parameters
                    .local_data_commitment
                    .parameters()
                    .clone(),
                local_data_root: local_data.local_data_root.clone(),
                position,
            };
            assert!(S::verify(&program_snark_pvk, &program_pub_input, &proof)?);
        }

        Ok(Self::PrivateWitness {
            verification_key: to_bytes![verification_key]?,
            proof: to_bytes![proof]?,
        })
    }

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
