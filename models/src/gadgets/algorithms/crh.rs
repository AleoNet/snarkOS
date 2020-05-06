use crate::{
    algorithms::CRH,
    curves::{Field, PrimeField},
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            boolean::Boolean,
            eq::{ConditionalEqGadget, EqGadget},
            select::CondSelectGadget,
            uint8::UInt8,
            ToBytesGadget,
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;

use std::fmt::Debug;

pub trait CRHGadget<H: CRH, F: Field>: Sized {
    type ParametersGadget: AllocGadget<H::Parameters, F> + Clone;
    type OutputGadget: ConditionalEqGadget<F>
        + EqGadget<F>
        + ToBytesGadget<F>
        + CondSelectGadget<F>
        + AllocGadget<H::Output, F>
        + Debug
        + Clone
        + Sized;

    fn check_evaluation_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError>;
}

pub trait MaskedCRHGadget<H: CRH, F: PrimeField>: CRHGadget<H, F> {
    fn extend_mask<CS: ConstraintSystem<F>>(mut cs: CS, mask: &[UInt8]) -> Result<Vec<UInt8>, SynthesisError> {
        let zero = [Boolean::constant(false), Boolean::constant(true)];
        let one = [Boolean::constant(true), Boolean::constant(false)];

        let mut extended_mask = vec![];
        for (i, m) in mask.iter().enumerate() {
            let m_bits = m.into_bits_le();
            for c in m_bits.iter().enumerate().collect::<Vec<_>>().chunks(4) {
                let mut new_byte = vec![];
                for (j, b) in c {
                    let bit1 = Boolean::conditionally_select(
                        cs.ns(|| format!("Extend bit {} in integer {}, bit 1", j, i)),
                        b,
                        &zero[0],
                        &one[0],
                    )?;
                    new_byte.push(bit1);
                    let bit2 = Boolean::conditionally_select(
                        cs.ns(|| format!("Extend bit {} in integer {}, bit 2", j, i)),
                        b,
                        &zero[1],
                        &one[1],
                    )?;
                    new_byte.push(bit2);
                }
                extended_mask.push(UInt8::from_bits_le(&new_byte));
            }
        }

        Ok(extended_mask)
    }

    fn check_evaluation_gadget_masked<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
        mask: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError>;
}
