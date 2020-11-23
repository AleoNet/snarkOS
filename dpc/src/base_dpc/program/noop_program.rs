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

use crate::base_dpc::{BaseDPCComponents, LocalData, NoopCircuit, PrivateProgramInput, ProgramLocalData};
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    algorithms::{CommitmentScheme, SNARK},
    dpc::{Program, Record},
};
use snarkos_utilities::{to_bytes, ToBytes};

use rand::Rng;
use std::marker::PhantomData;

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: BaseDPCComponents, S: SNARK"),
    Debug(bound = "C: BaseDPCComponents, S: SNARK"),
    PartialEq(bound = "C: BaseDPCComponents, S: SNARK"),
    Eq(bound = "C: BaseDPCComponents, S: SNARK")
)]
pub struct NoopProgram<C: BaseDPCComponents, S: SNARK> {
    #[derivative(Default(value = "vec![0u8; 48]"))]
    identity: Vec<u8>,
    _components: PhantomData<C>,
    _snark: PhantomData<S>,
}

impl<C: BaseDPCComponents, S: SNARK> NoopProgram<C, S> {
    pub fn new(identity: Vec<u8>) -> Self {
        Self {
            identity,
            _components: PhantomData,
            _snark: PhantomData,
        }
    }
}

impl<C: BaseDPCComponents, S: SNARK> Program for NoopProgram<C, S>
where
    S: SNARK<AssignedCircuit = NoopCircuit<C>, VerifierInput = ProgramLocalData<C>>,
{
    type LocalData = LocalData<C>;
    type PrivateWitness = PrivateProgramInput;
    type ProvingParameters = S::ProvingParameters;
    type PublicInput = ();
    type VerificationParameters = S::VerificationParameters;

    fn execute<R: Rng>(
        &self,
        proving_key: &Self::ProvingParameters,
        verification_key: &Self::VerificationParameters,
        local_data: &Self::LocalData,
        position: u8,
        rng: &mut R,
    ) -> Result<Self::PrivateWitness, DPCError> {
        let num_records = local_data.old_records.len() + local_data.new_records.len();
        assert!((position as usize) < num_records);

        let record = if (position as usize) < local_data.old_records.len() {
            &local_data.old_records[position as usize]
        } else {
            &local_data.new_records[position as usize - local_data.old_records.len()]
        };

        if (position as usize) < C::NUM_INPUT_RECORDS {
            assert_eq!(self.identity, record.death_program_id());
        } else {
            assert_eq!(self.identity, record.birth_program_id());
        }

        let local_data_root = local_data.local_data_merkle_tree.root();

        let circuit = NoopCircuit::<C>::new(&local_data.system_parameters, &local_data_root, position);

        let proof = S::prove(proving_key, &circuit, rng)?;

        {
            let program_snark_pvk: <S as SNARK>::PreparedVerificationParameters = verification_key.clone().into();

            let program_pub_input: ProgramLocalData<C> = ProgramLocalData {
                local_data_commitment_parameters: local_data
                    .system_parameters
                    .local_data_commitment
                    .parameters()
                    .clone(),
                local_data_root,
                position,
            };
            assert!(S::verify(&program_snark_pvk, &program_pub_input, &proof)?);
        }

        Ok(Self::PrivateWitness {
            verification_key: to_bytes![verification_key]?,
            proof: to_bytes![proof]?,
        })
    }

    fn evaluate(&self, _p: &Self::PublicInput, _w: &Self::PrivateWitness) -> bool {
        unimplemented!()
    }

    fn into_compact_repr(&self) -> Vec<u8> {
        self.identity.clone()
    }
}
