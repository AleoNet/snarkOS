use snarkos_errors::dpc::DPCError;

use rand::Rng;

pub trait Program: Clone {
    type LocalData;
    type PublicInput;
    type PrivateWitness;
    type ProvingParameters;
    type VerificationParameters;

    /// Executes and returns the program proof
    fn execute<R: Rng>(
        &self,
        proving_key: &Self::ProvingParameters,
        verification_key: &Self::VerificationParameters,
        local_data: &Self::LocalData,
        position: u8,
        rng: &mut R,
    ) -> Result<Self::PrivateWitness, DPCError>;

    /// Returns the evaluation of the program on given input and witness.
    fn evaluate(&self, primary: &Self::PublicInput, witness: &Self::PrivateWitness) -> bool;

    /// Returns the program identity
    fn into_compact_repr(&self) -> Vec<u8>;
}
