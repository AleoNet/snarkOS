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

use crate::base_dpc::BaseDPCComponents;
use snarkos_models::{algorithms::SNARK, parameters::Parameters};
use snarkos_parameters::*;
use snarkos_utilities::bytes::FromBytes;

use std::io::Result as IoResult;

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct SystemParameters<C: BaseDPCComponents> {
    pub account_commitment: C::AccountCommitment,
    pub account_encryption: C::AccountEncryption,
    pub account_signature: C::AccountSignature,
    pub record_commitment: C::RecordCommitment,
    pub encrypted_record_crh: C::EncryptedRecordCRH,
    pub inner_snark_verification_key_crh: C::InnerSNARKVerificationKeyCRH,
    pub program_verification_key_commitment: C::ProgramVerificationKeyCommitment,
    pub program_verification_key_crh: C::ProgramVerificationKeyCRH,
    pub local_data_crh: C::LocalDataCRH,
    pub local_data_commitment: C::LocalDataCommitment,
    pub serial_number_nonce: C::SerialNumberNonceCRH,
}

impl<C: BaseDPCComponents> SystemParameters<C> {
    // TODO (howardwu): Inspect what is going on with program_verification_key_commitment.
    pub fn load() -> IoResult<Self> {
        let account_commitment: C::AccountCommitment =
            From::from(FromBytes::read(AccountCommitmentParameters::load_bytes()?.as_slice())?);
        let account_encryption: C::AccountEncryption =
            From::from(FromBytes::read(AccountEncryptionParameters::load_bytes()?.as_slice())?);
        let account_signature: C::AccountSignature =
            From::from(FromBytes::read(AccountSignatureParameters::load_bytes()?.as_slice())?);
        let encrypted_record_crh: C::EncryptedRecordCRH =
            From::from(FromBytes::read(EncryptedRecordCRHParameters::load_bytes()?.as_slice())?);
        let inner_snark_verification_key_crh: C::InnerSNARKVerificationKeyCRH =
            From::from(FromBytes::read(InnerSNARKVKCRHParameters::load_bytes()?.as_slice())?);
        let local_data_crh: C::LocalDataCRH =
            From::from(FromBytes::read(LocalDataCRHParameters::load_bytes()?.as_slice())?);
        let local_data_commitment: C::LocalDataCommitment = From::from(FromBytes::read(
            LocalDataCommitmentParameters::load_bytes()?.as_slice(),
        )?);
        let program_verification_key_commitment: C::ProgramVerificationKeyCommitment =
            From::from(FromBytes::read(&[][..])?);
        let program_verification_key_crh: C::ProgramVerificationKeyCRH =
            From::from(FromBytes::read(ProgramVKCRHParameters::load_bytes()?.as_slice())?);
        let record_commitment: C::RecordCommitment =
            From::from(FromBytes::read(RecordCommitmentParameters::load_bytes()?.as_slice())?);
        let serial_number_nonce: C::SerialNumberNonceCRH = From::from(FromBytes::read(
            SerialNumberNonceCRHParameters::load_bytes()?.as_slice(),
        )?);

        Ok(Self {
            account_commitment,
            account_encryption,
            account_signature,
            encrypted_record_crh,
            inner_snark_verification_key_crh,
            local_data_crh,
            local_data_commitment,
            program_verification_key_commitment,
            program_verification_key_crh,
            record_commitment,
            serial_number_nonce,
        })
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct NoopProgramSNARKParameters<C: BaseDPCComponents> {
    pub proving_key: <C::NoopProgramSNARK as SNARK>::ProvingParameters,
    pub verification_key: <C::NoopProgramSNARK as SNARK>::VerificationParameters,
}

impl<C: BaseDPCComponents> NoopProgramSNARKParameters<C> {
    // TODO (howardwu): Why are we not preparing the VK here?
    pub fn load() -> IoResult<Self> {
        let proving_key: <C::NoopProgramSNARK as SNARK>::ProvingParameters =
            FromBytes::read(NoopProgramSNARKPKParameters::load_bytes()?.as_slice())?;
        let verification_key = <C::NoopProgramSNARK as SNARK>::VerificationParameters::read(
            NoopProgramSNARKVKParameters::load_bytes()?.as_slice(),
        )?;

        Ok(Self {
            proving_key,
            verification_key,
        })
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct PublicParameters<C: BaseDPCComponents> {
    pub system_parameters: SystemParameters<C>,
    pub noop_program_snark_parameters: NoopProgramSNARKParameters<C>,
    pub inner_snark_parameters: (
        Option<<C::InnerSNARK as SNARK>::ProvingParameters>,
        <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
    ),
    pub outer_snark_parameters: (
        Option<<C::OuterSNARK as SNARK>::ProvingParameters>,
        <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
    ),
}

impl<C: BaseDPCComponents> PublicParameters<C> {
    pub fn account_commitment_parameters(&self) -> &C::AccountCommitment {
        &self.system_parameters.account_commitment
    }

    pub fn account_encryption_parameters(&self) -> &C::AccountEncryption {
        &self.system_parameters.account_encryption
    }

    pub fn account_signature_parameters(&self) -> &C::AccountSignature {
        &self.system_parameters.account_signature
    }

    pub fn inner_snark_parameters(
        &self,
    ) -> &(
        Option<<C::InnerSNARK as SNARK>::ProvingParameters>,
        <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
    ) {
        &self.inner_snark_parameters
    }

    pub fn local_data_crh_parameters(&self) -> &C::LocalDataCRH {
        &self.system_parameters.local_data_crh
    }

    pub fn local_data_commitment_parameters(&self) -> &C::LocalDataCommitment {
        &self.system_parameters.local_data_commitment
    }

    pub fn outer_snark_parameters(
        &self,
    ) -> &(
        Option<<C::OuterSNARK as SNARK>::ProvingParameters>,
        <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
    ) {
        &self.outer_snark_parameters
    }

    pub fn noop_program_snark_parameters(&self) -> &NoopProgramSNARKParameters<C> {
        &self.noop_program_snark_parameters
    }

    pub fn program_verification_key_commitment_parameters(&self) -> &C::ProgramVerificationKeyCommitment {
        &self.system_parameters.program_verification_key_commitment
    }

    pub fn program_verification_key_crh_parameters(&self) -> &C::ProgramVerificationKeyCRH {
        &self.system_parameters.program_verification_key_crh
    }

    pub fn record_commitment_parameters(&self) -> &C::RecordCommitment {
        &self.system_parameters.record_commitment
    }

    pub fn encrypted_record_crh_parameters(&self) -> &C::EncryptedRecordCRH {
        &self.system_parameters.encrypted_record_crh
    }

    pub fn serial_number_nonce_parameters(&self) -> &C::SerialNumberNonceCRH {
        &self.system_parameters.serial_number_nonce
    }

    pub fn load(verify_only: bool) -> IoResult<Self> {
        let system_parameters = SystemParameters::<C>::load()?;
        let noop_program_snark_parameters = NoopProgramSNARKParameters::<C>::load()?;

        let inner_snark_parameters = {
            let inner_snark_pk = match verify_only {
                true => None,
                false => Some(<C::InnerSNARK as SNARK>::ProvingParameters::read(
                    InnerSNARKPKParameters::load_bytes()?.as_slice(),
                )?),
            };

            let inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters =
                <C::InnerSNARK as SNARK>::VerificationParameters::read(
                    InnerSNARKVKParameters::load_bytes()?.as_slice(),
                )?;

            (inner_snark_pk, inner_snark_vk.into())
        };

        let outer_snark_parameters = {
            let outer_snark_pk = match verify_only {
                true => None,
                false => Some(<C::OuterSNARK as SNARK>::ProvingParameters::read(
                    OuterSNARKPKParameters::load_bytes()?.as_slice(),
                )?),
            };

            let outer_snark_vk: <C::OuterSNARK as SNARK>::VerificationParameters =
                <C::OuterSNARK as SNARK>::VerificationParameters::read(
                    OuterSNARKVKParameters::load_bytes()?.as_slice(),
                )?;

            (outer_snark_pk, outer_snark_vk.into())
        };

        Ok(Self {
            system_parameters,
            noop_program_snark_parameters,
            inner_snark_parameters,
            outer_snark_parameters,
        })
    }

    pub fn load_vk_direct() -> IoResult<Self> {
        let system_parameters = SystemParameters::<C>::load()?;
        let noop_program_snark_parameters = NoopProgramSNARKParameters::<C>::load()?;

        let inner_snark_parameters = {
            let inner_snark_pk = None;
            let inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters =
                <C::InnerSNARK as SNARK>::VerificationParameters::read(
                    InnerSNARKVKParameters::load_bytes()?.as_slice(),
                )?;
            (inner_snark_pk, inner_snark_vk.into())
        };

        let outer_snark_parameters = {
            let outer_snark_pk = None;
            let outer_snark_vk: <C::OuterSNARK as SNARK>::VerificationParameters =
                <C::OuterSNARK as SNARK>::VerificationParameters::read(
                    OuterSNARKVKParameters::load_bytes()?.as_slice(),
                )?;
            (outer_snark_pk, outer_snark_vk.into())
        };

        Ok(Self {
            system_parameters,
            noop_program_snark_parameters,
            inner_snark_parameters,
            outer_snark_parameters,
        })
    }
}
