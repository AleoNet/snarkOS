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
    inner_circuit_gadget::execute_inner_proof_gadget,
    parameters::SystemParameters,
    record::DPCRecord,
    record_encryption::RecordEncryptionGadgetComponents,
    BaseDPCComponents,
};
use snarkos_algorithms::merkle_tree::{MerklePath, MerkleTreeDigest};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, EncryptionScheme, SignatureScheme, CRH},
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem},
};
use snarkos_objects::{AccountPrivateKey, AleoAmount};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct InnerCircuit<C: BaseDPCComponents> {
    // Parameters
    system_parameters: SystemParameters<C>,
    ledger_parameters: C::MerkleParameters,

    ledger_digest: MerkleTreeDigest<C::MerkleParameters>,

    // Inputs for old records.
    old_records: Vec<DPCRecord<C>>,
    old_witnesses: Vec<MerklePath<C::MerkleParameters>>,
    old_account_private_keys: Vec<AccountPrivateKey<C>>,
    old_serial_numbers: Vec<<C::AccountSignature as SignatureScheme>::PublicKey>,

    // Inputs for new records.
    new_records: Vec<DPCRecord<C>>,
    new_serial_number_nonce_randomness: Vec<[u8; 32]>,
    new_commitments: Vec<<C::RecordCommitment as CommitmentScheme>::Output>,

    // Inputs for encryption of new records.
    new_records_encryption_randomness: Vec<<C::AccountEncryption as EncryptionScheme>::Randomness>,
    new_records_encryption_gadget_components: Vec<RecordEncryptionGadgetComponents<C>>,
    new_encrypted_record_hashes: Vec<<C::EncryptedRecordCRH as CRH>::Output>,

    // Commitment to Programs and to local data.
    program_commitment: <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output,
    program_randomness: <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness,

    local_data_root: <C::LocalDataCRH as CRH>::Output,
    local_data_commitment_randomizers: Vec<<C::LocalDataCommitment as CommitmentScheme>::Randomness>,

    memo: [u8; 32],

    value_balance: AleoAmount,

    network_id: u8,
}

impl<C: BaseDPCComponents> InnerCircuit<C> {
    pub fn blank(system_parameters: &SystemParameters<C>, ledger_parameters: &C::MerkleParameters) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;
        let digest = MerkleTreeDigest::<C::MerkleParameters>::default();

        let old_serial_numbers =
            vec![<C::AccountSignature as SignatureScheme>::PublicKey::default(); num_input_records];
        let old_records = vec![DPCRecord::default(); num_input_records];
        let old_witnesses = vec![MerklePath::default(); num_input_records];
        let old_account_private_keys = vec![AccountPrivateKey::default(); num_input_records];

        let new_commitments = vec![<C::RecordCommitment as CommitmentScheme>::Output::default(); num_output_records];
        let new_serial_number_nonce_randomness = vec![[0u8; 32]; num_output_records];
        let new_records = vec![DPCRecord::default(); num_output_records];

        let new_records_encryption_randomness =
            vec![<C::AccountEncryption as EncryptionScheme>::Randomness::default(); num_output_records];

        let new_records_encryption_gadget_components =
            vec![RecordEncryptionGadgetComponents::<C>::default(); num_output_records];

        let new_encrypted_record_hashes = vec![<C::EncryptedRecordCRH as CRH>::Output::default(); num_output_records];

        let memo = [0u8; 32];

        let program_commitment = <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output::default();
        let program_randomness = <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness::default();

        let local_data_root = <C::LocalDataCRH as CRH>::Output::default();
        let local_data_commitment_randomizers = vec![
            <C::LocalDataCommitment as CommitmentScheme>::Randomness::default();
            num_input_records + num_output_records
        ];

        let value_balance = AleoAmount::ZERO;

        let network_id: u8 = 0;

        Self {
            // Parameters
            system_parameters: system_parameters.clone(),
            ledger_parameters: ledger_parameters.clone(),

            // Digest
            ledger_digest: digest,

            // Input records
            old_records,
            old_witnesses,
            old_account_private_keys,
            old_serial_numbers,

            // Output records
            new_records,
            new_serial_number_nonce_randomness,
            new_commitments,

            new_records_encryption_randomness,
            new_records_encryption_gadget_components,
            new_encrypted_record_hashes,

            // Other stuff
            program_commitment,
            program_randomness,
            local_data_root,
            local_data_commitment_randomizers,
            memo,
            value_balance,
            network_id,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        // Parameters
        system_parameters: SystemParameters<C>,
        ledger_parameters: C::MerkleParameters,

        // Digest
        ledger_digest: MerkleTreeDigest<C::MerkleParameters>,

        // Old records
        old_records: Vec<DPCRecord<C>>,
        old_witnesses: Vec<MerklePath<C::MerkleParameters>>,
        old_account_private_keys: Vec<AccountPrivateKey<C>>,
        old_serial_numbers: Vec<<C::AccountSignature as SignatureScheme>::PublicKey>,

        // New records
        new_records: Vec<DPCRecord<C>>,
        new_serial_number_nonce_randomness: Vec<[u8; 32]>,
        new_commitments: Vec<<C::RecordCommitment as CommitmentScheme>::Output>,

        new_records_encryption_randomness: Vec<<C::AccountEncryption as EncryptionScheme>::Randomness>,
        new_records_encryption_gadget_components: Vec<RecordEncryptionGadgetComponents<C>>,
        new_encrypted_record_hashes: Vec<<C::EncryptedRecordCRH as CRH>::Output>,

        // Other stuff
        program_commitment: <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output,
        program_randomness: <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness,

        local_data_root: <C::LocalDataCRH as CRH>::Output,
        local_data_commitment_randomizers: Vec<<C::LocalDataCommitment as CommitmentScheme>::Randomness>,

        memo: [u8; 32],

        value_balance: AleoAmount,

        network_id: u8,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        assert_eq!(num_input_records, old_records.len());
        assert_eq!(num_input_records, old_witnesses.len());
        assert_eq!(num_input_records, old_account_private_keys.len());
        assert_eq!(num_input_records, old_serial_numbers.len());

        assert_eq!(num_output_records, new_records.len());
        assert_eq!(num_output_records, new_serial_number_nonce_randomness.len());
        assert_eq!(num_output_records, new_commitments.len());

        assert_eq!(num_output_records, new_records_encryption_randomness.len());
        assert_eq!(num_output_records, new_records_encryption_gadget_components.len());
        assert_eq!(num_output_records, new_encrypted_record_hashes.len());

        // TODO (raychu86) Fix the lengths to be generic
        let record_encoding_length = 7;

        for gadget_components in &new_records_encryption_gadget_components {
            assert_eq!(gadget_components.record_field_elements.len(), record_encoding_length);
            assert_eq!(gadget_components.record_group_encoding.len(), record_encoding_length);
            assert_eq!(gadget_components.ciphertext_selectors.len(), record_encoding_length + 1);
            assert_eq!(gadget_components.fq_high_selectors.len(), record_encoding_length);
            assert_eq!(
                gadget_components.encryption_blinding_exponents.len(),
                record_encoding_length
            );
        }

        Self {
            // Parameters
            system_parameters,
            ledger_parameters,

            // Digest
            ledger_digest,

            // Input records
            old_records,
            old_witnesses,
            old_account_private_keys,
            old_serial_numbers,

            // Output records
            new_records,
            new_serial_number_nonce_randomness,
            new_commitments,

            new_records_encryption_randomness,
            new_records_encryption_gadget_components,
            new_encrypted_record_hashes,

            // Other stuff
            program_commitment,
            program_randomness,
            local_data_root,
            local_data_commitment_randomizers,
            memo,
            value_balance,
            network_id,
        }
    }
}

impl<C: BaseDPCComponents> ConstraintSynthesizer<C::InnerField> for InnerCircuit<C> {
    fn generate_constraints<CS: ConstraintSystem<C::InnerField>>(&self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_inner_proof_gadget::<C, CS>(
            cs,
            // Parameters
            &self.system_parameters,
            &self.ledger_parameters,
            // Digest
            &self.ledger_digest,
            // Old records
            &self.old_records,
            &self.old_witnesses,
            &self.old_account_private_keys,
            &self.old_serial_numbers,
            // New records
            &self.new_records,
            &self.new_serial_number_nonce_randomness,
            &self.new_commitments,
            &self.new_records_encryption_randomness,
            &self.new_records_encryption_gadget_components,
            &self.new_encrypted_record_hashes,
            // Other stuff
            &self.program_commitment,
            &self.program_randomness,
            &self.local_data_root,
            &self.local_data_commitment_randomizers,
            &self.memo,
            self.value_balance,
            self.network_id,
        )?;
        Ok(())
    }
}
