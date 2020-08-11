use crate::{
    base_dpc::{parameters::SystemParameters, record::DPCRecord, record_payload::RecordPayload, *},
    Assignment,
};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    dpc::Record,
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget},
        r1cs::{ConstraintSynthesizer, ConstraintSystem},
        utilities::{alloc::AllocGadget, eq::EqGadget, uint::UInt8},
    },
};
//use snarkos_gadgets::algorithms::merkle_tree::merkle_path::MerklePathGadget;

pub struct DummyCircuit<C: BaseDPCComponents> {
    /// System parameters
    pub system_parameters: Option<SystemParameters<C>>,

    /// Commitment to the program input.
    pub local_data_root: Option<<C::LocalDataCRH as CRH>::Output>,

    /// Record associated with the given position
    pub record: Option<DPCRecord<C>>,

    /// Local data commitment randomizer to derive the root
    pub local_data_commitment_randomizer: Option<<C::LocalDataCommitment as CommitmentScheme>::Randomness>,

    /// Record position
    pub position: u8,
}

impl<C: BaseDPCComponents> DummyCircuit<C> {
    pub fn blank(system_parameters: &SystemParameters<C>) -> Self {
        let local_data_root = <C::LocalDataCRH as CRH>::Output::default();
        let record = DPCRecord::default();
        let local_data_commitment_randomizer = <C::LocalDataCommitment as CommitmentScheme>::Randomness::default();

        Self {
            system_parameters: Some(system_parameters.clone()),
            local_data_root: Some(local_data_root),
            record: Some(record),
            local_data_commitment_randomizer: Some(local_data_commitment_randomizer),
            position: 0u8,
        }
    }

    pub fn new(local_data: &LocalData<C>, position: u8) -> Self {
        let records = [&local_data.old_records[..], &local_data.new_records[..]].concat();
        let record = &records[position as usize];
        let local_data_commitment_randomizer = &local_data.local_data_commitment_randomizers[position as usize];

        Self {
            system_parameters: Some(local_data.system_parameters.clone()),
            local_data_root: Some(local_data.local_data_merkle_tree.root()),
            record: Some(record.clone()),
            local_data_commitment_randomizer: Some(local_data_commitment_randomizer.clone()),
            position,
        }
    }
}

impl<C: BaseDPCComponents> ConstraintSynthesizer<C::InnerField> for DummyCircuit<C> {
    fn generate_constraints<CS: ConstraintSystem<C::InnerField>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_dummy_check_gadget(
            cs,
            self.system_parameters.get()?,
            self.local_data_root.get()?,
            self.record.get()?,
            self.local_data_commitment_randomizer.get()?,
            self.position,
        )
    }
}

fn execute_dummy_check_gadget<C: BaseDPCComponents, CS: ConstraintSystem<C::InnerField>>(
    cs: &mut CS,
    system_parameters: &SystemParameters<C>,
    local_data_root: &<C::LocalDataCRH as CRH>::Output,
    record: &DPCRecord<C>,
    local_data_commitment_randomizer: &<C::LocalDataCommitment as CommitmentScheme>::Randomness,
    position: u8,
) -> Result<(), SynthesisError> {
    // Allocate the position
    let _position = UInt8::alloc_input_vec(cs.ns(|| "Alloc position"), &[position])?;

    // Allocate the parameters and local data root
    let _local_data_commitment_parameters_gadget =
        <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare local data commitment parameters"),
            || Ok(system_parameters.local_data_commitment.parameters().clone()),
        )?;

    let _local_data_crh_parameters = <C::LocalDataCRHGadget as CRHGadget<_, _>>::ParametersGadget::alloc(
        &mut cs.ns(|| "Declare local data CRH parameters"),
        || Ok(system_parameters.local_data_crh.parameters()),
    )?;

    let _local_data_root_gadget = <C::LocalDataCRHGadget as CRHGadget<_, _>>::OutputGadget::alloc_input(
        cs.ns(|| "Allocate local data root"),
        || Ok(local_data_root),
    )?;

    // Enforce that the value is 0 and the payload is empty

    let zero_value = UInt8::constant_vec(&to_bytes![0u64]?);
    let empty_payload = UInt8::constant_vec(&to_bytes![RecordPayload::default()]?);

    let given_payload = UInt8::alloc_vec(&mut cs.ns(|| "given_payload"), &record.payload().to_bytes())?;
    let given_value = UInt8::alloc_vec(&mut cs.ns(|| "given_value"), &to_bytes![record.value()]?)?;

    given_value.enforce_equal(&mut cs.ns(|| "Enforce that the record has a zero value"), &zero_value)?;

    given_payload.enforce_equal(
        &mut cs.ns(|| "Enforce that the record has an empty payload"),
        &empty_payload,
    )?;

    // TODO (raychu86) Enforce that the local data commitment is valid for the root

    // Create the record commitment

    // Create the local data commitment

    let _commitment_randomness = <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::RandomnessGadget::alloc(
        cs.ns(|| "Allocate record local data commitment randomness"),
        || Ok(local_data_commitment_randomizer),
    )?;

    // Alloc the witness gadget. - Currently we do not have witnesses because the root is generated from scratch

    //    let witness_gadget = MerklePathGadget::<_, C::LocalDataCRH, _>::alloc(
    //        &mut cs.ns(|| "Declare local data membership witness"),
    //        || Ok(witness),
    //    )?;

    // Enforce that record commitment and witness is correct given the root

    //    witness_gadget.check_membership(
    //        &mut cs.ns(|| "Perform local data commitment membership witness check"),
    //        &local_data_crh_parameters,
    //        &local_data_root_gadget,
    //        &candidate_local_data_commitment,
    //    )?;

    Ok(())
}
