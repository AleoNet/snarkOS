pub trait Program: Clone {
    type PublicInput;
    type PrivateWitness;

    /// Returns the evaluation of the program on given input and witness.
    fn evaluate(&self, primary: &Self::PublicInput, witness: &Self::PrivateWitness) -> bool;

    fn into_compact_repr(&self) -> Vec<u8>;
}
