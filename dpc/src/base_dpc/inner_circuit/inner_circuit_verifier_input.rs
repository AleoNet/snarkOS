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

use crate::base_dpc::{parameters::SystemParameters, BaseDPCComponents};
use snarkos_algorithms::merkle_tree::MerkleTreeDigest;
use snarkos_errors::curves::ConstraintFieldError;
use snarkos_models::{
    algorithms::{CommitmentScheme, EncryptionScheme, MerkleParameters, SignatureScheme, CRH},
    curves::to_field_vec::ToConstraintField,
};
use snarkos_objects::AleoAmount;

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct InnerCircuitVerifierInput<C: BaseDPCComponents> {
    // Commitment, CRH, and signature parameters
    pub system_parameters: SystemParameters<C>,

    // Ledger parameters and digest
    pub ledger_parameters: C::MerkleParameters,
    pub ledger_digest: MerkleTreeDigest<C::MerkleParameters>,

    // Input record serial numbers
    pub old_serial_numbers: Vec<<C::AccountSignature as SignatureScheme>::PublicKey>,

    // Output record commitments
    pub new_commitments: Vec<<C::RecordCommitment as CommitmentScheme>::Output>,

    // New encrypted record hashes
    pub new_encrypted_record_hashes: Vec<<C::EncryptedRecordCRH as CRH>::Output>,

    // Program input commitment and local data root
    pub program_commitment: <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output,
    pub local_data_root: <C::LocalDataCRH as CRH>::Output,

    pub memo: [u8; 32],
    pub value_balance: AleoAmount,
    pub network_id: u8,
}

impl<C: BaseDPCComponents> ToConstraintField<C::InnerField> for InnerCircuitVerifierInput<C>
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
    fn to_field_elements(&self) -> Result<Vec<C::InnerField>, ConstraintFieldError> {
        let mut v = Vec::new();

        v.extend_from_slice(
            &self
                .system_parameters
                .account_commitment
                .parameters()
                .to_field_elements()?,
        );
        v.extend_from_slice(
            &self
                .system_parameters
                .account_encryption
                .parameters()
                .to_field_elements()?,
        );
        v.extend_from_slice(
            &self
                .system_parameters
                .account_signature
                .parameters()
                .to_field_elements()?,
        );
        v.extend_from_slice(
            &self
                .system_parameters
                .record_commitment
                .parameters()
                .to_field_elements()?,
        );
        v.extend_from_slice(
            &self
                .system_parameters
                .encrypted_record_crh
                .parameters()
                .to_field_elements()?,
        );
        v.extend_from_slice(
            &self
                .system_parameters
                .program_verification_key_commitment
                .parameters()
                .to_field_elements()?,
        );
        v.extend_from_slice(&self.system_parameters.local_data_crh.parameters().to_field_elements()?);
        v.extend_from_slice(
            &self
                .system_parameters
                .serial_number_nonce
                .parameters()
                .to_field_elements()?,
        );

        v.extend_from_slice(&self.ledger_parameters.parameters().to_field_elements()?);
        v.extend_from_slice(&self.ledger_digest.to_field_elements()?);

        for sn in &self.old_serial_numbers {
            v.extend_from_slice(&sn.to_field_elements()?);
        }

        for (cm, encrypted_record_hash) in self.new_commitments.iter().zip(&self.new_encrypted_record_hashes) {
            v.extend_from_slice(&cm.to_field_elements()?);
            v.extend_from_slice(&encrypted_record_hash.to_field_elements()?);
        }

        v.extend_from_slice(&self.program_commitment.to_field_elements()?);
        v.extend_from_slice(&ToConstraintField::<C::InnerField>::to_field_elements(&self.memo)?);
        v.extend_from_slice(&ToConstraintField::<C::InnerField>::to_field_elements(
            &[self.network_id][..],
        )?);
        v.extend_from_slice(&self.local_data_root.to_field_elements()?);

        v.extend_from_slice(&ToConstraintField::<C::InnerField>::to_field_elements(
            &self.value_balance.0.to_le_bytes()[..],
        )?);
        Ok(v)
    }
}
