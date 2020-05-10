use crate::{
    base_dpc::{parameters::CircuitParameters, *},
    Assignment,
};
use snarkos_errors::{curves::ConstraintFieldError, gadgets::SynthesisError};
use snarkos_models::{
    algorithms::CommitmentScheme,
    curves::to_field_vec::ToConstraintField,
    gadgets::{
        algorithms::CommitmentGadget,
        r1cs::{ConstraintSynthesizer, ConstraintSystem},
        utilities::{alloc::AllocGadget, eq::EqGadget, uint8::UInt8},
    },
};

pub struct PaymentPredicateLocalData<C: BaseDPCComponents> {
    pub local_data_commitment_parameters: <C::LocalDataCommitment as CommitmentScheme>::Parameters,
    pub local_data_commitment: <C::LocalDataCommitment as CommitmentScheme>::Output,
    pub value_commitment_parameters: <C::ValueCommitment as CommitmentScheme>::Parameters,
    pub value_commitment_randomness: <C::ValueCommitment as CommitmentScheme>::Randomness,
    pub value_commitment: <C::ValueCommitment as CommitmentScheme>::Output,
    pub position: u8,
}

/// Convert each component to bytes and pack into field elements.
impl<C: BaseDPCComponents> ToConstraintField<C::InnerField> for PaymentPredicateLocalData<C>
where
    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::LocalDataCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
    <C::ValueCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::ValueCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
{
    fn to_field_elements(&self) -> Result<Vec<C::InnerField>, ConstraintFieldError> {
        let mut v = ToConstraintField::<C::InnerField>::to_field_elements(&[self.position][..])?;

        v.extend_from_slice(&self.local_data_commitment_parameters.to_field_elements()?);
        v.extend_from_slice(&self.local_data_commitment.to_field_elements()?);
        v.extend_from_slice(&self.value_commitment_parameters.to_field_elements()?);
        v.extend(ToConstraintField::<C::InnerField>::to_field_elements(
            &to_bytes![self.value_commitment_randomness]?[..],
        )?);
        v.extend_from_slice(&self.value_commitment.to_field_elements()?);
        Ok(v)
    }
}

pub struct PaymentCircuit<C: BaseDPCComponents> {
    pub circuit_parameters: Option<CircuitParameters<C>>,

    pub local_data_commitment: Option<<C::LocalDataCommitment as CommitmentScheme>::Output>,
    pub value_commitment_randomness: Option<<C::ValueCommitment as CommitmentScheme>::Randomness>,
    pub value_commitment: Option<<C::ValueCommitment as CommitmentScheme>::Output>,

    pub position: u8,
    pub value: u64,
}

impl<C: BaseDPCComponents> PaymentCircuit<C> {
    pub fn blank(circuit_parameters: &CircuitParameters<C>) -> Self {
        let local_data_commitment = <C::LocalDataCommitment as CommitmentScheme>::Output::default();
        let value_commitment_randomness = <C::ValueCommitment as CommitmentScheme>::Randomness::default();
        let value_commitment = <C::ValueCommitment as CommitmentScheme>::Output::default();

        Self {
            circuit_parameters: Some(circuit_parameters.clone()),
            value_commitment_randomness: Some(value_commitment_randomness),
            local_data_commitment: Some(local_data_commitment),
            value_commitment: Some(value_commitment),
            position: 0u8,
            value: 0,
        }
    }

    pub fn new(
        circuit_parameters: &CircuitParameters<C>,
        local_data_commitment: &<C::LocalDataCommitment as CommitmentScheme>::Output,
        value_commitment_randomness: &<C::ValueCommitment as CommitmentScheme>::Randomness,
        value_commitment: &<C::ValueCommitment as CommitmentScheme>::Output,
        position: u8,
        value: u64,
    ) -> Self {
        Self {
            circuit_parameters: Some(circuit_parameters.clone()),
            local_data_commitment: Some(local_data_commitment.clone()),
            value_commitment_randomness: Some(value_commitment_randomness.clone()),

            value_commitment: Some(value_commitment.clone()),
            position,
            value,
        }
    }
}

impl<C: BaseDPCComponents> ConstraintSynthesizer<C::InnerField> for PaymentCircuit<C> {
    fn generate_constraints<CS: ConstraintSystem<C::InnerField>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_payment_check_gadget(
            cs,
            self.circuit_parameters.get()?,
            self.local_data_commitment.get()?,
            self.value_commitment.get()?,
            self.value_commitment_randomness.get()?,
            self.position,
            self.value,
        )
    }
}

fn execute_payment_check_gadget<C: BaseDPCComponents, CS: ConstraintSystem<C::InnerField>>(
    cs: &mut CS,
    circuit_parameters: &CircuitParameters<C>,
    local_data_commitment: &<C::LocalDataCommitment as CommitmentScheme>::Output,
    value_commitment: &<C::ValueCommitment as CommitmentScheme>::Output,
    value_commitment_randomness: &<C::ValueCommitment as CommitmentScheme>::Randomness,
    position: u8,
    value: u64,
) -> Result<(), SynthesisError> {
    let _position = UInt8::alloc_input_vec(cs.ns(|| "Alloc position"), &[position])?;

    let _local_data_commitment_parameters_gadget =
        <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare Pred Input Comm parameters"),
            || Ok(circuit_parameters.local_data_commitment.parameters().clone()),
        )?;

    let _local_data_commitment_gadget =
        <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::OutputGadget::alloc_input(
            cs.ns(|| "Allocate local data commitment"),
            || Ok(local_data_commitment),
        )?;

    let value_commitment_parameters_gadget =
        <C::ValueCommitmentGadget as CommitmentGadget<_, _>>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare value comm parameters"),
            || Ok(circuit_parameters.value_commitment.parameters()),
        )?;

    let value_commitment_randomness_gadget =
        <C::ValueCommitmentGadget as CommitmentGadget<_, _>>::RandomnessGadget::alloc_input(
            cs.ns(|| "Allocate value commitment randomness"),
            || Ok(value_commitment_randomness),
        )?;

    let declared_value_commitment_gadget =
        <C::ValueCommitmentGadget as CommitmentGadget<_, _>>::OutputGadget::alloc_input(
            cs.ns(|| "Allocate declared value commitment"),
            || Ok(value_commitment),
        )?;

    let value_input = UInt8::alloc_vec(cs.ns(|| "Alloc value"), &value.to_le_bytes())?;

    let computed_value_commitment_gadget = C::ValueCommitmentGadget::check_commitment_gadget(
        cs.ns(|| "Generate value commitment"),
        &value_commitment_parameters_gadget,
        &value_input,
        &value_commitment_randomness_gadget,
    )?;

    // Check that the value commitments are equivalent
    computed_value_commitment_gadget.enforce_equal(
        &mut cs.ns(|| "Check that declared and computed value commitments are equal"),
        &declared_value_commitment_gadget,
    )?;

    Ok(())
}
