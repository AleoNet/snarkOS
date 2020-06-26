use crate::algorithms::crh::pedersen::PedersenCRHParametersGadget;
use snarkos_algorithms::crh::{BoweHopwoodPedersenCRH, PedersenSize, BOWE_HOPWOOD_CHUNK_SIZE};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group},
    gadgets::{
        algorithms::CRHGadget,
        curves::GroupGadget,
        r1cs::ConstraintSystem,
        utilities::{
            boolean::Boolean,
            uint::unsigned_integer::{UInt, UInt8},
        },
    },
};

use std::marker::PhantomData;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoweHopwoodPedersenCRHGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    _group: PhantomData<*const G>,
    _group_gadget: PhantomData<*const GG>,
    _engine: PhantomData<F>,
}

impl<F: Field, G: Group, GG: GroupGadget<G, F>, S: PedersenSize> CRHGadget<BoweHopwoodPedersenCRH<G, S>, F>
    for BoweHopwoodPedersenCRHGadget<G, F, GG>
{
    type OutputGadget = GG;
    type ParametersGadget = PedersenCRHParametersGadget<G, S, F, GG>;

    fn check_evaluation_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError> {
        // Pad the input if it is not the current length.
        let mut input_in_bits: Vec<_> = input.to_vec().into_iter().flat_map(|byte| byte.to_bits_le()).collect();
        if (input_in_bits.len()) % BOWE_HOPWOOD_CHUNK_SIZE != 0 {
            let current_length = input_in_bits.len();
            for _ in 0..(BOWE_HOPWOOD_CHUNK_SIZE - current_length % BOWE_HOPWOOD_CHUNK_SIZE) {
                input_in_bits.push(Boolean::constant(false));
            }
        }
        assert!(input_in_bits.len() % BOWE_HOPWOOD_CHUNK_SIZE == 0);
        assert_eq!(parameters.parameters.bases.len(), S::NUM_WINDOWS);
        for generators in parameters.parameters.bases.iter() {
            assert_eq!(generators.len(), S::WINDOW_SIZE);
        }

        // Allocate new variable for the result.
        let input_in_bits = input_in_bits
            .chunks(S::WINDOW_SIZE * BOWE_HOPWOOD_CHUNK_SIZE)
            .map(|x| x.chunks(BOWE_HOPWOOD_CHUNK_SIZE).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let result =
            GG::precomputed_base_3_bit_signed_digit_scalar_mul(cs, &parameters.parameters.bases, &input_in_bits)?;

        Ok(result)
    }
}
