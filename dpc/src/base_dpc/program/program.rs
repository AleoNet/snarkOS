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
use snarkos_errors::curves::ConstraintFieldError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    curves::to_field_vec::ToConstraintField,
};

/// Program verification key and proof
/// Represented as bytes to be generic for any Program SNARK
pub struct PrivateProgramInput {
    pub verification_key: Vec<u8>,
    pub proof: Vec<u8>,
}

impl Clone for PrivateProgramInput {
    fn clone(&self) -> Self {
        Self {
            verification_key: self.verification_key.clone(),
            proof: self.proof.clone(),
        }
    }
}

pub struct ProgramLocalData<C: BaseDPCComponents> {
    pub local_data_commitment_parameters: <C::LocalDataCommitment as CommitmentScheme>::Parameters,
    // TODO (raychu86) add local_data_crh_parameters
    pub local_data_root: <C::LocalDataCRH as CRH>::Output,
    pub position: u8,
}

/// Convert each component to bytes and pack into field elements.
impl<C: BaseDPCComponents> ToConstraintField<C::InnerField> for ProgramLocalData<C>
where
    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::LocalDataCRH as CRH>::Output: ToConstraintField<C::InnerField>,
{
    fn to_field_elements(&self) -> Result<Vec<C::InnerField>, ConstraintFieldError> {
        let mut v = ToConstraintField::<C::InnerField>::to_field_elements(&[self.position][..])?;

        v.extend_from_slice(&self.local_data_commitment_parameters.to_field_elements()?);
        v.extend_from_slice(&self.local_data_root.to_field_elements()?);
        Ok(v)
    }
}
