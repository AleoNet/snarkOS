use marlin::Marlin;

use snarkos_models::{
    algorithms::SNARK,
    curves::{to_field_vec::ToConstraintField, PairingEngine},
    gadgets::r1cs::ConstraintSynthesizer,
};

use snarkos_errors::algorithms::SNARKError;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::{thread_rng, Rng};
use std::marker::PhantomData;

use blake2::Blake2s;
use poly_commit::marlin_pc::MarlinKZG10;

type MultiPC<E> = MarlinKZG10<E>;
type MarlinInst<E> = Marlin<<E as PairingEngine>::Fr, MultiPC<E>, Blake2s>;

/// Note: V should serialize its contents to `Vec<E::Fr>` in the same order as
/// during the constraint generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarlinSnark<'a, E: PairingEngine, C: ConstraintSynthesizer<E::Fr>, V: ToConstraintField<E::Fr> + ?Sized> {
    _engine: PhantomData<E>,
    _circuit: PhantomData<C>,
    _verifier_input: PhantomData<V>,
    _key_liftime: PhantomData<&'a marlin::IndexProverKey<'a, E::Fr, MultiPC<E>, C>>,
}

pub struct Parameters<'a, E: PairingEngine, C: ConstraintSynthesizer<E::Fr>> {
    pub index_prover_key: marlin::IndexProverKey<'a, <E as PairingEngine>::Fr, MultiPC<E>, C>,
    pub index_verifier_key: marlin::IndexVerifierKey<<E as PairingEngine>::Fr, MultiPC<E>, C>,
}

impl<'a, E: PairingEngine, C: ConstraintSynthesizer<E::Fr>> Parameters<'a, E, C> {
    pub fn new<R: Rng>(circuit: C, rng: &mut R) -> Self {
        let universal_srs = Box::new(MarlinInst::universal_setup(10000, 10000, 100000, rng).unwrap());
        let universal_srs = Box::leak(universal_srs);
        let (index_pk, index_vk) = MarlinInst::index(universal_srs, circuit).unwrap();
        let params = Self {
            index_prover_key: index_pk,
            index_verifier_key: index_vk,
        };
        params
    }
}

impl<'a, E: PairingEngine, C: ConstraintSynthesizer<E::Fr>> FromBytes for Parameters<'a, E, C> {
    fn read<R: snarkos_utilities::io::Read>(_: R) -> snarkos_utilities::io::Result<Self> {
        Err(snarkos_utilities::io::ErrorKind::NotFound.into())
    }
}

impl<'a, E: PairingEngine, C: ConstraintSynthesizer<E::Fr>> ToBytes for Parameters<'a, E, C> {
    fn write<W: snarkos_utilities::io::Write>(&self, _: W) -> snarkos_utilities::io::Result<()> {
        Err(snarkos_utilities::io::ErrorKind::NotFound.into())
    }
}

impl<'a, E: PairingEngine, C: ConstraintSynthesizer<E::Fr>> From<Parameters<'a, E, C>>
    for marlin::IndexVerifierKey<E::Fr, MultiPC<E>, C>
{
    fn from(keys: Parameters<'a, E, C>) -> Self {
        keys.index_verifier_key
    }
}

impl<'a, E: PairingEngine, C: ConstraintSynthesizer<E::Fr>> Clone for Parameters<'a, E, C> {
    fn clone(&self) -> Self {
        Self {
            index_prover_key: self.index_prover_key.clone(),
            index_verifier_key: self.index_verifier_key.clone(),
        }
    }
}

impl<'a, E: PairingEngine, C: ConstraintSynthesizer<E::Fr>, V: ToConstraintField<E::Fr> + ?Sized> SNARK
    for MarlinSnark<'a, E, C, V>
{
    type AssignedCircuit = C;
    type Circuit = C;
    type PreparedVerificationParameters = marlin::IndexVerifierKey<<E as PairingEngine>::Fr, MultiPC<E>, C>;
    type Proof = marlin::Proof<E::Fr, MultiPC<E>, C>;
    type ProvingParameters = Parameters<'a, E, C>;
    type VerificationParameters = marlin::IndexVerifierKey<<E as PairingEngine>::Fr, MultiPC<E>, C>;
    type VerifierInput = V;

    fn setup<R: Rng>(
        circuit: Self::Circuit,
        rng: &mut R,
    ) -> Result<(Self::ProvingParameters, Self::PreparedVerificationParameters), SNARKError> {
        let parameters = Parameters::<'a, E, C>::new(circuit, rng);
        let verifier_key = parameters.index_verifier_key.clone();
        Ok((parameters, verifier_key))
    }

    fn prove<R: Rng>(
        pp: &Self::ProvingParameters,
        circuit: Self::AssignedCircuit,
        rng: &mut R,
    ) -> Result<Self::Proof, SNARKError> {
        let proof = MarlinInst::prove(&pp.index_prover_key, circuit, rng).unwrap();
        Ok(proof)
    }

    fn verify(
        vk: &Self::PreparedVerificationParameters,
        input: &Self::VerifierInput,
        proof: &Self::Proof,
    ) -> Result<bool, SNARKError> {
        let rng = &mut thread_rng();
        MarlinInst::verify(&vk, &input.to_field_elements().unwrap(), &proof, rng).unwrap();

        Ok(true)
    }
}
