use crate::{
    base_dpc::{parameters::SystemParameters, BaseDPCComponents},
    Assignment,
};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget},
        r1cs::{ConstraintSynthesizer, ConstraintSystem},
        utilities::{alloc::AllocGadget, uint::UInt8},
    },
};

/// Always-accept program
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

    let _local_data_root_gadget = <C::LocalDataCRHGadget as CRHGadget<_, _>>::OutputGadget::alloc_input(
        cs.ns(|| "Allocate local data root"),
        || Ok(local_data_root),
    )?;

    Ok(())
}
