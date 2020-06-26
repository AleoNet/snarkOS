use crate::{
    base_dpc::{parameters::CircuitParameters, *},
    Assignment,
};
use snarkos_errors::{curves::ConstraintFieldError, gadgets::SynthesisError};
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    curves::to_field_vec::ToConstraintField,
    dpc::DPCComponents,
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget},
        r1cs::{ConstraintSynthesizer, ConstraintSystem},
        utilities::{alloc::AllocGadget, uint::UInt8},
    },
};

pub struct PredicateLocalData<C: BaseDPCComponents> {
    pub local_data_commitment_parameters: <C::LocalDataCommitment as CommitmentScheme>::Parameters,
    pub local_data_commitment: <<C as DPCComponents>::LocalDataMerkleCommitment as CRH>::Output,
    pub position: u8,
}

/// Convert each component to bytes and pack into field elements.
impl<C: BaseDPCComponents> ToConstraintField<C::InnerField> for PredicateLocalData<C>
where
    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <<C as DPCComponents>::LocalDataMerkleCommitment as CRH>::Output: ToConstraintField<C::InnerField>,
{
    fn to_field_elements(&self) -> Result<Vec<C::InnerField>, ConstraintFieldError> {
        let mut v = ToConstraintField::<C::InnerField>::to_field_elements(&[self.position][..])?;

        v.extend_from_slice(&self.local_data_commitment_parameters.to_field_elements()?);
        v.extend_from_slice(&self.local_data_commitment.to_field_elements()?);
        Ok(v)
    }
}

pub struct PredicateCircuit<C: BaseDPCComponents> {
    // Parameters
    pub circuit_parameters: Option<CircuitParameters<C>>,

    // Commitment to Predicate input.
    pub local_data_commitment: Option<<<C as DPCComponents>::LocalDataMerkleCommitment as CRH>::Output>,
    pub position: u8,
}

impl<C: BaseDPCComponents> PredicateCircuit<C> {
    pub fn blank(circuit_parameters: &CircuitParameters<C>) -> Self {
        let local_data_commitment = <<C as DPCComponents>::LocalDataMerkleCommitment as CRH>::Output::default();

        Self {
            circuit_parameters: Some(circuit_parameters.clone()),
            local_data_commitment: Some(local_data_commitment),
            position: 0u8,
        }
    }

    pub fn new(
        circuit_parameters: &CircuitParameters<C>,
        local_data_commitment: &<<C as DPCComponents>::LocalDataMerkleCommitment as CRH>::Output,
        position: u8,
    ) -> Self {
        Self {
            circuit_parameters: Some(circuit_parameters.clone()),
            local_data_commitment: Some(local_data_commitment.clone()),
            position,
        }
    }
}

impl<C: BaseDPCComponents> ConstraintSynthesizer<C::InnerField> for PredicateCircuit<C> {
    fn generate_constraints<CS: ConstraintSystem<C::InnerField>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_predicate_check_gadget(
            cs,
            self.circuit_parameters.get()?,
            self.local_data_commitment.get()?,
            self.position,
        )
    }
}

fn execute_predicate_check_gadget<C: BaseDPCComponents, CS: ConstraintSystem<C::InnerField>>(
    cs: &mut CS,
    circuit_parameters: &CircuitParameters<C>,
    local_data_commitment: &<<C as DPCComponents>::LocalDataMerkleCommitment as CRH>::Output,
    position: u8,
) -> Result<(), SynthesisError> {
    let _position = UInt8::alloc_input_vec(cs.ns(|| "Alloc position"), &[position])?;

    let _local_data_commitment_parameters_gadget =
        <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare Pred Input Comm parameters"),
            || Ok(circuit_parameters.local_data_commitment.parameters().clone()),
        )?;

    let _local_data_commitment_gadget =
        <<C as DPCComponents>::LocalDataMerkleCommitmentGadget as CRHGadget<_, _>>::OutputGadget::alloc_input(
            &mut cs.ns(|| "Declare local data commitment digest"),
            || Ok(local_data_commitment),
        )?;

    Ok(())
}
