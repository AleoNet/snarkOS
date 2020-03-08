use snarkos_algorithms::crh::{PedersenCRH, PedersenCRHParameters, PedersenSize};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group},
    gadgets::{
        algorithms::CRHGadget,
        curves::GroupGadget,
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, uint8::UInt8},
    },
};

use std::{borrow::Borrow, marker::PhantomData};

#[derive(Clone)]
pub struct PedersenCRHParametersGadget<G: Group, S: PedersenSize, F: Field, GG: GroupGadget<G, F>> {
    parameters: PedersenCRHParameters<G, S>,
    _group: PhantomData<GG>,
    _engine: PhantomData<F>,
}

impl<G: Group, S: PedersenSize, F: Field, GG: GroupGadget<G, F>> AllocGadget<PedersenCRHParameters<G, S>, F>
    for PedersenCRHParametersGadget<G, S, F, GG>
{
    fn alloc<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<PedersenCRHParameters<G, S>>,
        CS: ConstraintSystem<F>,
    >(
        _cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(PedersenCRHParametersGadget {
            parameters: value_gen()?.borrow().clone(),
            _group: PhantomData,
            _engine: PhantomData,
        })
    }

    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<PedersenCRHParameters<G, S>>,
        CS: ConstraintSystem<F>,
    >(
        _cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(PedersenCRHParametersGadget {
            parameters: value_gen()?.borrow().clone(),
            _group: PhantomData,
            _engine: PhantomData,
        })
    }
}

pub struct PedersenCRHGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    _group: PhantomData<*const G>,
    _group_gadget: PhantomData<*const GG>,
    _engine: PhantomData<F>,
}

impl<F: Field, G: Group, GG: GroupGadget<G, F>, S: PedersenSize> CRHGadget<PedersenCRH<G, S>, F>
    for PedersenCRHGadget<G, F, GG>
{
    type OutputGadget = GG;
    type ParametersGadget = PedersenCRHParametersGadget<G, S, F, GG>;

    fn check_evaluation_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError> {
        let mut padded_input = input.to_vec();
        // Pad the input if it is not the current length.
        if input.len() * 8 < S::WINDOW_SIZE * S::NUM_WINDOWS {
            let current_length = input.len();
            for _ in current_length..(S::WINDOW_SIZE * S::NUM_WINDOWS / 8) {
                padded_input.push(UInt8::constant(0u8));
            }
        }
        assert_eq!(padded_input.len() * 8, S::WINDOW_SIZE * S::NUM_WINDOWS);
        assert_eq!(parameters.parameters.bases.len(), S::NUM_WINDOWS);

        // Allocate new variable for the result.
        let input_in_bits: Vec<_> = padded_input.iter().flat_map(|byte| byte.into_bits_le()).collect();
        let input_in_bits = input_in_bits.chunks(S::WINDOW_SIZE);

        Ok(GG::precomputed_base_multiscalar_mul(
            cs,
            &parameters.parameters.bases,
            input_in_bits,
        )?)
    }
}

#[cfg(test)]
mod test {

}
