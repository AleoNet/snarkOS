use snarkos_errors::algorithms::SNARKError;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::Rng;

pub trait SNARK {
    type AssignedCircuit;
    type Circuit;
    type Proof: ToBytes + FromBytes + Clone + Default;
    type PreparedVerificationParameters: Clone + Default + From<Self::VerificationParameters>;
    type ProvingParameters: Clone;
    type VerificationParameters: Clone + Default + From<Self::PreparedVerificationParameters>;
    type VerifierInput: ?Sized;

    fn setup<R: Rng>(
        circuit: Self::Circuit,
        rng: &mut R,
    ) -> Result<(Self::ProvingParameters, Self::PreparedVerificationParameters), SNARKError>;

    fn prove<R: Rng>(
        parameter: &Self::ProvingParameters,
        input_and_witness: Self::AssignedCircuit,
        rng: &mut R,
    ) -> Result<Self::Proof, SNARKError>;

    fn verify(
        verifier_key: &Self::PreparedVerificationParameters,
        input: &Self::VerifierInput,
        proof: &Self::Proof,
    ) -> Result<bool, SNARKError>;
}
