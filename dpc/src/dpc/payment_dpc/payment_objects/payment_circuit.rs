//use crate::{
//    constraints::Assignment,
//    payment_dpc::{parameters::CommAndCRHPublicParameters, *},
//};
//
//use snarkos_errors::{curves::ConstraintFieldError, gadgets::SynthesisError};
//use snarkos_models::{
//    algorithms::CommitmentScheme,
//    curves::to_field_vec::ToConstraintField,
//    gadgets::{
//        algorithms::CommitmentGadget,
//        r1cs::{ConstraintSynthesizer, ConstraintSystem},
//        utilities::{alloc::AllocGadget, eq::EqGadget, uint8::UInt8},
//    },
//};
//
//pub struct PaymentPredicateLocalData<C: PlainDPCComponents> {
//    pub local_data_comm_pp: <C::LocalDataComm as CommitmentScheme>::Parameters,
//    pub local_data_comm: <C::LocalDataComm as CommitmentScheme>::Output,
//    pub value_comm_pp: <C::ValueComm as CommitmentScheme>::Parameters,
//    pub value_commitment: <C::ValueComm as CommitmentScheme>::Output,
//    pub position: u8,
//    //    pub value: u64,
//}
//
///// Convert each component to bytes and pack into field elements.
//impl<C: PlainDPCComponents> ToConstraintField<C::CoreCheckF> for PaymentPredicateLocalData<C>
//where
//    <C::LocalDataComm as CommitmentScheme>::Parameters: ToConstraintField<C::CoreCheckF>,
//    <C::LocalDataComm as CommitmentScheme>::Output:
//        ToConstraintField<C::CoreCheckF>,
//    <C::ValueComm as CommitmentScheme>::Parameters: ToConstraintField<C::CoreCheckF>,
//    <C::ValueComm as CommitmentScheme>::Output:
//        ToConstraintField<C::CoreCheckF>,
//{
//    fn to_field_elements(&self) -> Result<Vec<C::CoreCheckF>, ConstraintFieldError> {
//        let mut v = ToConstraintField::<C::CoreCheckF>::to_field_elements(&[self.position][..])?;
//
//        v.extend_from_slice(&self.local_data_comm_pp.to_field_elements()?);
//        v.extend_from_slice(&self.local_data_comm.to_field_elements()?);
//        v.extend_from_slice(&self.value_comm_pp.to_field_elements()?);
//        v.extend_from_slice(&self.value_commitment.to_field_elements()?);
//        Ok(v)
//    }
//}
//
//pub struct PaymentCircuit<C: PlainDPCComponents> {
//    pub parameters: Option<CommAndCRHPublicParameters<C>>,
//
//    pub local_data_comm: Option<<C::LocalDataComm as CommitmentScheme>::Output>,
//    pub value_commitment_randomness: Option<<C::ValueComm as CommitmentScheme>::Randomness>,
//    pub value_commitment: Option<<C::ValueComm as CommitmentScheme>::Output>,
//
//    pub position: u8,
//    pub value: u64,
//}
//
//impl<C: PlainDPCComponents> PaymentCircuit<C> {
//    pub fn blank(comm_and_crh_parameters: &CommAndCRHPublicParameters<C>) -> Self {
//        let local_data_comm =
//            <C::LocalDataComm as CommitmentScheme>::Output::default();
//        let value_commitment_randomness = <C::ValueComm as CommitmentScheme>::Randomness::default();
//        let value_commitment =
//            <C::ValueComm as CommitmentScheme>::Output::default();
//
//        Self {
//            parameters: Some(comm_and_crh_parameters.clone()),
//            value_commitment_randomness: Some(value_commitment_randomness),
//            local_data_comm: Some(local_data_comm),
//            value_commitment: Some(value_commitment),
//            position: 0u8,
//            value: 0,
//        }
//    }
//
//    pub fn new(
//        comm_amd_crh_parameters: &CommAndCRHPublicParameters<C>,
//        local_data_comm: &<C::LocalDataComm as CommitmentScheme>::Output,
//        value_commitment_randomness: &<C::ValueComm as CommitmentScheme>::Randomness,
//        value_commitment: &<C::ValueComm as CommitmentScheme>::Output,
//        position: u8,
//        value: u64,
//    ) -> Self {
//        Self {
//            parameters: Some(comm_amd_crh_parameters.clone()),
//            local_data_comm: Some(local_data_comm.clone()),
//            value_commitment_randomness: Some(value_commitment_randomness.clone()),
//
//            value_commitment: Some(value_commitment.clone()),
//            position,
//            value,
//        }
//    }
//}
//
//impl<C: PlainDPCComponents> ConstraintSynthesizer<C::CoreCheckF> for PaymentCircuit<C> {
//    fn generate_constraints<CS: ConstraintSystem<C::CoreCheckF>>(
//        self,
//        cs: &mut CS,
//    ) -> Result<(), SynthesisError> {
//        execute_payment_check_gadget(
//            cs,
//            self.parameters.get()?,
//            self.local_data_comm.get()?,
//            self.value_commitment.get()?,
//            self.value_commitment_randomness.get()?,
//            self.position,
//            self.value,
//        )
//    }
//}
//
//fn execute_payment_check_gadget<C: PlainDPCComponents, CS: ConstraintSystem<C::CoreCheckF>>(
//    cs: &mut CS,
//    comm_and_crh_parameters: &CommAndCRHPublicParameters<C>,
//    local_data_commitment: &<C::LocalDataComm as CommitmentScheme>::Output,
//    value_commitment: &<C::ValueComm as CommitmentScheme>::Output,
//    value_commitment_randomness: &<C::ValueComm as CommitmentScheme>::Randomness,
//    position: u8,
//    value: u64,
//) -> Result<(), SynthesisError> {
//    let _position = UInt8::alloc_input_vec(cs.ns(|| "Alloc position"), &[position])?;
//
//    let _local_data_comm_pp = <C::LocalDataCommGadget as CommitmentGadget<_, _>>::ParametersGadget::alloc_input(
//        &mut cs.ns(|| "Declare Pred Input Comm parameters"),
//        || Ok(comm_and_crh_parameters.local_data_comm_pp.parameters().clone()),
//    )?;
//
//    let _local_data_comm =
//        <C::LocalDataCommGadget as CommitmentGadget<_, _>>::OutputGadget::alloc_input(
//            cs.ns(|| "Allocate local data commitment"),
//            || Ok(local_data_commitment),
//        )?;
//
//    let value_comm_randomness =
//        <C::ValueCommGadget as CommitmentGadget<_, _>>::RandomnessGadget::alloc(
//            cs.ns(|| "Allocate value commitment randomness"),
//            || Ok(value_commitment_randomness),
//        )?;
//
//    let value_comm_pp =
//        <C::ValueCommGadget as CommitmentGadget<_, _>>::ParametersGadget::alloc_input(
//            &mut cs.ns(|| "Declare value comm parameters"),
//            || Ok(comm_and_crh_parameters.value_comm_pp.parameters()),
//        )?;
//
//    let declared_value_commitment =
//        <C::ValueCommGadget as CommitmentGadget<_, _>>::OutputGadget::alloc_input(
//            cs.ns(|| "Allocate declared value commitment"),
//            || Ok(value_commitment),
//        )?;
//
//    let value_input = UInt8::alloc_vec(cs.ns(|| "Alloc value"), &value.to_le_bytes())?;
//
//    let computed_value_commitment = C::ValueCommGadget::check_compressed_commitment_gadget(
//        cs.ns(|| "Generate value commitment"),
//        &value_comm_pp,
//        &value_input,
//        &value_comm_randomness,
//    )?;
//
//    // Check that the value commitments are equivalent
//    computed_value_commitment.enforce_equal(
//        &mut cs.ns(|| "Check that declared and computed value commitments are equal"),
//        &declared_value_commitment,
//    )?;
//
//    Ok(())
//}
