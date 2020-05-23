use crate::{
    curves::Field,
    gadgets::{r1cs::ConstraintSystem, utilities::boolean::Boolean},
};
use snarkvm_errors::gadgets::SynthesisError;

/// If condition is `true`, return `first`; else, select `second`.
pub trait CondSelectGadget<F: Field>
where
    Self: Sized,
{
    fn conditionally_select<CS: ConstraintSystem<F>>(
        cs: CS,
        cond: &Boolean,
        first: &Self,
        second: &Self,
    ) -> Result<Self, SynthesisError>;

    fn cost() -> usize;
}

/// Uses two bits to perform a lookup into a table
pub trait TwoBitLookupGadget<F: Field>
where
    Self: Sized,
{
    type TableConstant;
    fn two_bit_lookup<CS: ConstraintSystem<F>>(
        cs: CS,
        bits: &[Boolean],
        constants: &[Self::TableConstant],
    ) -> Result<Self, SynthesisError>;

    fn cost() -> usize;
}

/// Uses three bits to perform a lookup into a table, where the last bit
/// performs negation
pub trait ThreeBitCondNegLookupGadget<F: Field>
where
    Self: Sized,
{
    type TableConstant;
    fn three_bit_cond_neg_lookup<CS: ConstraintSystem<F>>(
        cs: CS,
        bits: &[Boolean],
        b0b1: &Boolean,
        constants: &[Self::TableConstant],
    ) -> Result<Self, SynthesisError>;

    fn cost() -> usize;
}
