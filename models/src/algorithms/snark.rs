use rand::Rng;
use snarkos_errors::algorithms::Error;
use snarkos_utilities::bytes::ToBytes;

pub trait SNARK {
    type Circuit;
    type AssignedCircuit;
    type ProvingParameters: Clone;
    type Proof: ToBytes + Clone + Default;
    type VerificationParameters: Clone + Default + From<Self::PreparedVerificationParameters>;
    type PreparedVerificationParameters: Clone + Default + From<Self::VerificationParameters>;
    type VerifierInput: ?Sized;

    fn setup<R: Rng>(
        circuit: Self::Circuit,
        rng: &mut R,
    ) -> Result<(Self::ProvingParameters, Self::PreparedVerificationParameters), Error>;

    fn prove<R: Rng>(
        parameter: &Self::ProvingParameters,
        input_and_witness: Self::AssignedCircuit,
        rng: &mut R,
    ) -> Result<Self::Proof, Error>;

    fn verify(
        verifier_key: &Self::PreparedVerificationParameters,
        input: &Self::VerifierInput,
        proof: &Self::Proof,
    ) -> Result<bool, Error>;
}
