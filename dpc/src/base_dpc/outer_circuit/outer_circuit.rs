// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::base_dpc::{
    outer_circuit_gadget::execute_outer_proof_gadget,
    parameters::SystemParameters,
    program::PrivateProgramInput,
    BaseDPCComponents,
};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, EncryptionScheme, MerkleParameters, SignatureScheme, CRH, SNARK},
    curves::to_field_vec::ToConstraintField,
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem},
};
use snarkos_objects::AleoAmount;
use snarkvm_algorithms::merkle_tree::MerkleTreeDigest;

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct OuterCircuit<C: BaseDPCComponents> {
    system_parameters: SystemParameters<C>,

    // Inner snark verifier public inputs
    ledger_parameters: C::MerkleParameters,
    ledger_digest: MerkleTreeDigest<C::MerkleParameters>,
    old_serial_numbers: Vec<<C::AccountSignature as SignatureScheme>::PublicKey>,
    new_commitments: Vec<<C::RecordCommitment as CommitmentScheme>::Output>,
    new_encrypted_record_hashes: Vec<<C::EncryptedRecordCRH as CRH>::Output>,
    memo: [u8; 32],
    value_balance: AleoAmount,
    network_id: u8,

    // Inner snark verifier private inputs
    inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters,
    inner_snark_proof: <C::InnerSNARK as SNARK>::Proof,

    old_private_program_inputs: Vec<PrivateProgramInput>,
    new_private_program_inputs: Vec<PrivateProgramInput>,

    program_commitment: <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output,
    program_randomness: <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness,
    local_data_root: <C::LocalDataCRH as CRH>::Output,

    inner_snark_id: <C::InnerSNARKVerificationKeyCRH as CRH>::Output,
}

impl<C: BaseDPCComponents> OuterCircuit<C> {
    pub fn blank(
        system_parameters: SystemParameters<C>,
        ledger_parameters: C::MerkleParameters,
        inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters,
        inner_snark_proof: <C::InnerSNARK as SNARK>::Proof,
        program_snark_vk_and_proof: PrivateProgramInput,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        let ledger_digest = MerkleTreeDigest::<C::MerkleParameters>::default();
        let old_serial_numbers =
            vec![<C::AccountSignature as SignatureScheme>::PublicKey::default(); num_input_records];
        let new_commitments = vec![<C::RecordCommitment as CommitmentScheme>::Output::default(); num_output_records];
        let new_encrypted_record_hashes = vec![<C::EncryptedRecordCRH as CRH>::Output::default(); num_output_records];
        let memo = [0u8; 32];
        let value_balance = AleoAmount::ZERO;
        let network_id = 0;

        let old_private_program_inputs = vec![program_snark_vk_and_proof.clone(); num_input_records];
        let new_private_program_inputs = vec![program_snark_vk_and_proof; num_output_records];

        let program_commitment = <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output::default();
        let program_randomness = <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness::default();
        let local_data_root = <C::LocalDataCRH as CRH>::Output::default();

        let inner_snark_id = <C::InnerSNARKVerificationKeyCRH as CRH>::Output::default();

        Self {
            system_parameters,
            ledger_parameters,
            ledger_digest,
            old_serial_numbers,
            new_commitments,
            memo,
            new_encrypted_record_hashes,
            value_balance,
            network_id,
            inner_snark_vk,
            inner_snark_proof,
            old_private_program_inputs,
            new_private_program_inputs,
            program_commitment,
            program_randomness,
            local_data_root,
            inner_snark_id,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        system_parameters: SystemParameters<C>,

        // Inner SNARK public inputs
        ledger_parameters: C::MerkleParameters,
        ledger_digest: MerkleTreeDigest<C::MerkleParameters>,
        old_serial_numbers: Vec<<C::AccountSignature as SignatureScheme>::PublicKey>,
        new_commitments: Vec<<C::RecordCommitment as CommitmentScheme>::Output>,
        new_encrypted_record_hashes: Vec<<C::EncryptedRecordCRH as CRH>::Output>,
        memo: [u8; 32],
        value_balance: AleoAmount,
        network_id: u8,

        // Inner SNARK private inputs
        inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters,
        inner_snark_proof: <C::InnerSNARK as SNARK>::Proof,

        // Private program input = Verification key and input
        // Commitment contains commitment to hash of death program vk.
        old_private_program_inputs: Vec<PrivateProgramInput>,

        // Private program input = Verification key and input
        // Commitment contains commitment to hash of birth program vk.
        new_private_program_inputs: Vec<PrivateProgramInput>,

        program_commitment: <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output,
        program_randomness: <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness,
        local_data_root: <C::LocalDataCRH as CRH>::Output,

        // Inner SNARK ID
        inner_snark_id: <C::InnerSNARKVerificationKeyCRH as CRH>::Output,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        assert_eq!(num_input_records, old_private_program_inputs.len());
        assert_eq!(num_output_records, new_private_program_inputs.len());
        assert_eq!(num_output_records, new_commitments.len());
        assert_eq!(num_output_records, new_encrypted_record_hashes.len());

        Self {
            system_parameters,
            ledger_parameters,
            ledger_digest,
            old_serial_numbers,
            new_commitments,
            new_encrypted_record_hashes,
            memo,
            value_balance,
            network_id,
            inner_snark_vk,
            inner_snark_proof,
            old_private_program_inputs,
            new_private_program_inputs,
            program_commitment,
            program_randomness,
            local_data_root,
            inner_snark_id,
        }
    }
}

impl<C: BaseDPCComponents> ConstraintSynthesizer<C::OuterField> for OuterCircuit<C>
where
    <C::AccountCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::AccountCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::AccountEncryption as EncryptionScheme>::Parameters: ToConstraintField<C::InnerField>,

    <C::AccountSignature as SignatureScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::AccountSignature as SignatureScheme>::PublicKey: ToConstraintField<C::InnerField>,

    <C::RecordCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::RecordCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::EncryptedRecordCRH as CRH>::Parameters: ToConstraintField<C::InnerField>,
    <C::EncryptedRecordCRH as CRH>::Output: ToConstraintField<C::InnerField>,

    <C::SerialNumberNonceCRH as CRH>::Parameters: ToConstraintField<C::InnerField>,

    <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::LocalDataCRH as CRH>::Parameters: ToConstraintField<C::InnerField>,
    <C::LocalDataCRH as CRH>::Output: ToConstraintField<C::InnerField>,

    <<C::MerkleParameters as MerkleParameters>::H as CRH>::Parameters: ToConstraintField<C::InnerField>,
    MerkleTreeDigest<C::MerkleParameters>: ToConstraintField<C::InnerField>,
{
    fn generate_constraints<CS: ConstraintSystem<C::OuterField>>(&self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_outer_proof_gadget::<C, CS>(
            cs,
            &self.system_parameters,
            &self.ledger_parameters,
            &self.ledger_digest,
            &self.old_serial_numbers,
            &self.new_commitments,
            &self.new_encrypted_record_hashes,
            &self.memo,
            self.value_balance,
            self.network_id,
            &self.inner_snark_vk,
            &self.inner_snark_proof,
            &self.old_private_program_inputs,
            &self.new_private_program_inputs,
            &self.program_commitment,
            &self.program_randomness,
            &self.local_data_root,
            &self.inner_snark_id,
        )?;
        Ok(())
    }
}
