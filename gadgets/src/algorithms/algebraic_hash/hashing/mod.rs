use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::Field,
    gadgets::{curves::FieldGadget, r1cs::ConstraintSystem},
};

pub mod poseidon;

pub trait PermutationGadget<F, FG>
where
    F: Field,
    FG: FieldGadget<F, F>,
{
    fn permute<CS: ConstraintSystem<F>>(&self, cs: CS, state: &mut [FG]) -> Result<(), SynthesisError>;
}
