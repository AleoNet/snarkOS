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
        let records = [&local_data.old_records[..], &local_data.new_records[..]].concat();
        assert!((position as usize) < records.len());

        let record = &records[position as usize];

        if (position as usize) < C::NUM_INPUT_RECORDS {
            assert_eq!(self.identity, record.death_program_id());
        } else {
            assert_eq!(self.identity, record.birth_program_id());
        }

        let local_data_root = local_data.local_data_merkle_tree.root();

        let circuit = NoopCircuit::<C>::new(&local_data.system_parameters, &local_data_root, position);

        let proof = S::prove(proving_key, circuit, rng)?;

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
