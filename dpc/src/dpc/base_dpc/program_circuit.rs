use crate::{
    base_dpc::{parameters::SystemParameters, *},
    Assignment,
};
use snarkos_errors::{curves::ConstraintFieldError, gadgets::SynthesisError};
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    curves::to_field_vec::ToConstraintField,
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget},
        r1cs::{ConstraintSynthesizer, ConstraintSystem},
        utilities::{alloc::AllocGadget, uint::UInt8},
    },
};

pub struct ProgramLocalData<C: BaseDPCComponents> {
    pub local_data_commitment_parameters: <C::LocalDataCommitment as CommitmentScheme>::Parameters,
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

pub struct ProgramCircuit<C: BaseDPCComponents> {
    /// System parameters
    pub system_parameters: Option<SystemParameters<C>>,

    /// Commitment to the program input.
    pub local_data_root: Option<<C::LocalDataCRH as CRH>::Output>,

    /// Record position
    pub position: u8,
}

impl<C: BaseDPCComponents> ProgramCircuit<C> {
    pub fn blank(system_parameters: &SystemParameters<C>) -> Self {
        let local_data_root = <C::LocalDataCRH as CRH>::Output::default();

        Self {
            system_parameters: Some(system_parameters.clone()),
            local_data_root: Some(local_data_root),
            position: 0u8,
        }
    }

    pub fn new(
        system_parameters: &SystemParameters<C>,
        local_data_root: &<C::LocalDataCRH as CRH>::Output,
        position: u8,
    ) -> Self {
        Self {
            system_parameters: Some(system_parameters.clone()),
            local_data_root: Some(local_data_root.clone()),
            position,
        }
    }
}

impl<C: BaseDPCComponents> ConstraintSynthesizer<C::InnerField> for ProgramCircuit<C> {
    fn generate_constraints<CS: ConstraintSystem<C::InnerField>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_payment_check_gadget(
            cs,
            self.system_parameters.get()?,
            self.local_data_root.get()?,
            self.position,
        )
    }
}

fn execute_payment_check_gadget<C: BaseDPCComponents, CS: ConstraintSystem<C::InnerField>>(
    cs: &mut CS,
    system_parameters: &SystemParameters<C>,
    local_data_root: &<C::LocalDataCRH as CRH>::Output,
    position: u8,
) -> Result<(), SynthesisError> {
    let _position = UInt8::alloc_input_vec(cs.ns(|| "Alloc position"), &[position])?;

    let _local_data_commitment_parameters_gadget =
        <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare local data commitment parameters"),
            || Ok(system_parameters.local_data_commitment.parameters().clone()),
        )?;

    let _local_data_commitment_gadget = <C::LocalDataCRHGadget as CRHGadget<_, _>>::OutputGadget::alloc_input(
        cs.ns(|| "Allocate local data root"),
        || Ok(local_data_root),
    )?;

    Ok(())
}
