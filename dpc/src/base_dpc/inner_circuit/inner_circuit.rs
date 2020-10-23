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

use crate::{
    base_dpc::{
        inner_circuit_gadget::execute_inner_proof_gadget,
        parameters::SystemParameters,
        record::DPCRecord,
        record_encryption::RecordEncryptionGadgetComponents,
        BaseDPCComponents,
    },
    Assignment,
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
    system_parameters: Option<SystemParameters<C>>,
    ledger_parameters: Option<C::MerkleParameters>,

    ledger_digest: Option<MerkleTreeDigest<C::MerkleParameters>>,

    // Inputs for old records.
    old_records: Option<Vec<DPCRecord<C>>>,
    old_witnesses: Option<Vec<MerklePath<C::MerkleParameters>>>,
    old_account_private_keys: Option<Vec<AccountPrivateKey<C>>>,
    old_serial_numbers: Option<Vec<<C::AccountSignature as SignatureScheme>::PublicKey>>,

    // Inputs for new records.
    new_records: Option<Vec<DPCRecord<C>>>,
    new_serial_number_nonce_randomness: Option<Vec<[u8; 32]>>,
    new_commitments: Option<Vec<<C::RecordCommitment as CommitmentScheme>::Output>>,

    // Inputs for encryption of new records.
    new_records_encryption_randomness: Option<Vec<<C::AccountEncryption as EncryptionScheme>::Randomness>>,
    new_records_encryption_gadget_components: Option<Vec<RecordEncryptionGadgetComponents<C>>>,
    new_encrypted_record_hashes: Option<Vec<<C::EncryptedRecordCRH as CRH>::Output>>,

    // Commitment to Programs and to local data.
    program_commitment: Option<<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output>,
    program_randomness: Option<<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness>,

    local_data_root: Option<<C::LocalDataCRH as CRH>::Output>,
    local_data_commitment_randomizers: Option<Vec<<C::LocalDataCommitment as CommitmentScheme>::Randomness>>,

    memo: Option<[u8; 32]>,

    value_balance: Option<AleoAmount>,

    network_id: Option<u8>,
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
            system_parameters: Some(system_parameters.clone()),
            ledger_parameters: Some(ledger_parameters.clone()),

            // Digest
            ledger_digest: Some(digest),

            // Input records
            old_records: Some(old_records),
            old_witnesses: Some(old_witnesses),
            old_account_private_keys: Some(old_account_private_keys),
            old_serial_numbers: Some(old_serial_numbers),

            // Output records
            new_records: Some(new_records),
            new_serial_number_nonce_randomness: Some(new_serial_number_nonce_randomness),
            new_commitments: Some(new_commitments),

            new_records_encryption_randomness: Some(new_records_encryption_randomness),
            new_records_encryption_gadget_components: Some(new_records_encryption_gadget_components),
            new_encrypted_record_hashes: Some(new_encrypted_record_hashes),

            // Other stuff
            program_commitment: Some(program_commitment),
            program_randomness: Some(program_randomness),
            local_data_root: Some(local_data_root),
            local_data_commitment_randomizers: Some(local_data_commitment_randomizers),
            memo: Some(memo),

            value_balance: Some(value_balance),

            network_id: Some(network_id),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        // Parameters
        system_parameters: &SystemParameters<C>,
        ledger_parameters: &C::MerkleParameters,

        // Digest
        ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,

        // Old records
        old_records: &[DPCRecord<C>],
        old_witnesses: &[MerklePath<C::MerkleParameters>],
        old_account_private_keys: &[AccountPrivateKey<C>],
        old_serial_numbers: &[<C::AccountSignature as SignatureScheme>::PublicKey],

        // New records
        new_records: &[DPCRecord<C>],
        new_serial_number_nonce_randomness: &[[u8; 32]],
        new_commitments: &[<C::RecordCommitment as CommitmentScheme>::Output],

        new_records_encryption_randomness: &[<C::AccountEncryption as EncryptionScheme>::Randomness],
        new_records_encryption_gadget_components: &[RecordEncryptionGadgetComponents<C>],
        new_encrypted_record_hashes: &[<C::EncryptedRecordCRH as CRH>::Output],

        // Other stuff
        program_commitment: &<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output,
        program_randomness: &<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness,

        local_data_root: &<C::LocalDataCRH as CRH>::Output,
        local_data_commitment_randomizers: &[<C::LocalDataCommitment as CommitmentScheme>::Randomness],

        memo: &[u8; 32],

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

        for gadget_components in new_records_encryption_gadget_components {
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
            system_parameters: Some(system_parameters.clone()),
            ledger_parameters: Some(ledger_parameters.clone()),

            // Digest
            ledger_digest: Some(ledger_digest.clone()),

            // Input records
            old_records: Some(old_records.to_vec()),
            old_witnesses: Some(old_witnesses.to_vec()),
            old_account_private_keys: Some(old_account_private_keys.to_vec()),
            old_serial_numbers: Some(old_serial_numbers.to_vec()),

            // Output records
            new_records: Some(new_records.to_vec()),
            new_serial_number_nonce_randomness: Some(new_serial_number_nonce_randomness.to_vec()),
            new_commitments: Some(new_commitments.to_vec()),

            new_records_encryption_randomness: Some(new_records_encryption_randomness.to_vec()),
            new_records_encryption_gadget_components: Some(new_records_encryption_gadget_components.to_vec()),
            new_encrypted_record_hashes: Some(new_encrypted_record_hashes.to_vec()),

            // Other stuff
            program_commitment: Some(program_commitment.clone()),
            program_randomness: Some(program_randomness.clone()),

            local_data_root: Some(local_data_root.clone()),
            local_data_commitment_randomizers: Some(local_data_commitment_randomizers.to_vec()),

            memo: Some(*memo),

            value_balance: Some(value_balance),

            network_id: Some(network_id),
        }
    }
}

impl<C: BaseDPCComponents> ConstraintSynthesizer<C::InnerField> for InnerCircuit<C> {
    fn generate_constraints<CS: ConstraintSystem<C::InnerField>>(&self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_inner_proof_gadget::<C, CS>(
            cs,
            // Parameters
            self.system_parameters.get()?,
            self.ledger_parameters.get()?,
            // Digest
            self.ledger_digest.get()?,
            // Old records
            self.old_records.get()?,
            self.old_witnesses.get()?,
            self.old_account_private_keys.get()?,
            self.old_serial_numbers.get()?,
            // New records
            self.new_records.get()?,
            self.new_serial_number_nonce_randomness.get()?,
            self.new_commitments.get()?,
            self.new_records_encryption_randomness.get()?,
            self.new_records_encryption_gadget_components.get()?,
            self.new_encrypted_record_hashes.get()?,
            // Other stuff
            self.program_commitment.get()?,
            self.program_randomness.get()?,
            self.local_data_root.get()?,
            self.local_data_commitment_randomizers.get()?,
            self.memo.get()?,
            *self.value_balance.get()?,
            *self.network_id.get()?,
        )?;
        Ok(())
    }
}
