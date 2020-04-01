use crate::{
    dpc::base_dpc::{
        inner_circuit::InnerCircuit,
        inner_circuit_verifier_input::InnerCircuitVerifierInput,
        outer_circuit::OuterCircuit,
        outer_circuit_verifier_input::OuterCircuitVerifierInput,
        payment_circuit::{PaymentCircuit, PaymentPredicateLocalData},
        predicate::DPCPredicate,
        transaction::DPCTransaction,
        BaseDPCComponents,
        LocalData as DPCLocalData,
        DPC,
    },
    ledger::ledger::BasicLedger,
};
use snarkos_algorithms::{
    commitment::{Blake2sCommitment, PedersenCompressedCommitment},
    crh::{PedersenCompressedCRH, PedersenSize},
    merkle_tree::MerkleParameters,
    prf::Blake2s,
    signature::SchnorrSignature,
    snark::GM17,
};
use snarkos_curves::{
    bls12_377::{fq::Fq as Bls12_377Fq, fr::Fr as Bls12_377Fr, Bls12_377},
    edwards_bls12::{EdwardsAffine, EdwardsProjective as EdwardsBls},
    edwards_sw6::EdwardsProjective as EdwardsSW,
    sw6::SW6,
};
use snarkos_gadgets::{
    algorithms::{
        binding_signature::BindingSignatureVerificationGadget,
        commitment::{Blake2sCommitmentGadget, PedersenCompressedCommitmentGadget},
        crh::PedersenCompressedCRHGadget,
        prf::Blake2sGadget,
        signature::SchnorrPublicKeyRandomizationGadget,
        snark::GM17VerifierGadget,
    },
    curves::{bls12_377::PairingGadget, edwards_bls12::EdwardsBlsGadget, edwards_sw6::EdwardsSWGadget},
};
use snarkos_models::{algorithms::CRH, dpc::DPCComponents};

use blake2::Blake2s as Blake2sHash;

pub const NUM_INPUT_RECORDS: usize = 2;
pub const NUM_OUTPUT_RECORDS: usize = 2;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SnNonceWindow;

// `WINDOW_SIZE * NUM_WINDOWS` = NUM_INPUT_RECORDS * 64 + 1 + 32 = 225 bytes
const SN_NONCE_SIZE_BITS: usize = NUM_INPUT_RECORDS * 2 * 512 + 8 + 256;
impl PedersenSize for SnNonceWindow {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = SN_NONCE_SIZE_BITS / 8;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PredVkHashWindow;

impl PedersenSize for PredVkHashWindow {
    const NUM_WINDOWS: usize = 38;
    const WINDOW_SIZE: usize = 300;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LocalDataWindow;

impl PedersenSize for LocalDataWindow {
    const NUM_WINDOWS: usize = 36;
    const WINDOW_SIZE: usize = 248;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TwoToOneWindow;
// `WINDOW_SIZE * NUM_WINDOWS` = 2 * 256 bits
impl PedersenSize for TwoToOneWindow {
    const NUM_WINDOWS: usize = 4;
    const WINDOW_SIZE: usize = 128;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RecordWindow;
impl PedersenSize for RecordWindow {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 225;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AddressWindow;
impl PedersenSize for AddressWindow {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 192;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ValueWindow;

impl PedersenSize for ValueWindow {
    const NUM_WINDOWS: usize = 4;
    // TODO fix window size inconsistency -
    //  Will fail binding signature test a % of the time
    //  when WINDOW_SIZE is smaller (128, 248, etc.)
    const WINDOW_SIZE: usize = 350;
}

type H = MerkleTreeCRH;

#[derive(Clone, PartialEq, Eq)]
pub struct CommitmentMerkleParameters(H);

impl MerkleParameters for CommitmentMerkleParameters {
    type H = H;

    const HEIGHT: usize = 32;

    fn crh(&self) -> &Self::H {
        &self.0
    }
}

impl Default for CommitmentMerkleParameters {
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        Self(H::setup(&mut rng))
    }
}

pub struct Components;

impl DPCComponents for Components {
    type AddressCommitment = AddressCommitment;
    type AddressCommitmentGadget = AddressCommitmentGadget;
    type InnerField = InnerField;
    type LocalDataCommitment = LocalDataCommitment;
    type LocalDataCommitmentGadget = LocalDataCommitmentGadget;
    type OuterField = OuterField;
    type PRF = PRF;
    type PRFGadget = PRFGadget;
    type PredicateVerificationKeyCommitment = PredicateVerificationKeyCommitment;
    type PredicateVerificationKeyCommitmentGadget = PredicateVerificationKeyCommitmentGadget;
    type PredicateVerificationKeyHash = PredicateVerificationKeyHash;
    type PredicateVerificationKeyHashGadget = PredicateVerificationKeyHashGadget;
    type RecordCommitment = RecordCommitment;
    type RecordCommitmentGadget = RecordCommitmentGadget;
    type SerialNumberNonce = SerialNumberNonce;
    type SerialNumberNonceGadget = SerialNumberNonceGadget;
    type Signature = Signature;
    type SignatureGadget = SignatureGadget;

    const NUM_INPUT_RECORDS: usize = NUM_INPUT_RECORDS;
    const NUM_OUTPUT_RECORDS: usize = NUM_OUTPUT_RECORDS;
}

impl BaseDPCComponents for Components {
    type BindingSignatureGadget = BindingSignatureGadget;
    type BindingSignatureGroup = EdwardsBls;
    type InnerSNARK = CoreCheckNIZK;
    type MerkleHashGadget = MerkleTreeCRHGadget;
    type MerkleParameters = CommitmentMerkleParameters;
    type OuterSNARK = ProofCheckNIZK;
    type PredicateSNARK = PredicateSNARK<Self>;
    type PredicateSNARKGadget = PredicateSNARKGadget;
    type ValueCommitment = ValueCommitment;
    type ValueCommitmentGadget = ValueCommitmentGadget;
}

// Native primitives

pub type InnerPairing = Bls12_377;
pub type OuterPairing = SW6;
pub type InnerField = Bls12_377Fr;
pub type OuterField = Bls12_377Fq;

pub type AddressCommitment = PedersenCompressedCommitment<EdwardsBls, AddressWindow>;
pub type RecordCommitment = PedersenCompressedCommitment<EdwardsBls, RecordWindow>;
pub type PredicateVerificationKeyCommitment = Blake2sCommitment;
pub type LocalDataCommitment = PedersenCompressedCommitment<EdwardsBls, LocalDataWindow>;
pub type ValueCommitment = PedersenCompressedCommitment<EdwardsBls, ValueWindow>;

pub type Signature = SchnorrSignature<EdwardsAffine, Blake2sHash>;

pub type MerkleTreeCRH = PedersenCompressedCRH<EdwardsBls, TwoToOneWindow>;
pub type SerialNumberNonce = PedersenCompressedCRH<EdwardsBls, SnNonceWindow>;
pub type PredicateVerificationKeyHash = PedersenCompressedCRH<EdwardsSW, PredVkHashWindow>;

pub type Predicate = DPCPredicate<Components>;
pub type CoreCheckNIZK = GM17<InnerPairing, InnerCircuit<Components>, InnerCircuitVerifierInput<Components>>;
pub type ProofCheckNIZK = GM17<OuterPairing, OuterCircuit<Components>, OuterCircuitVerifierInput<Components>>;
pub type PredicateSNARK<C> = GM17<InnerPairing, PaymentCircuit<C>, PaymentPredicateLocalData<C>>;
pub type PRF = Blake2s;

// Gadgets

pub type AddressCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type RecordCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type PredicateVerificationKeyCommitmentGadget = Blake2sCommitmentGadget;
pub type LocalDataCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type ValueCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;

pub type BindingSignatureGadget = BindingSignatureVerificationGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type SignatureGadget = SchnorrPublicKeyRandomizationGadget<EdwardsAffine, InnerField, EdwardsBlsGadget>;

pub type MerkleTreeCRHGadget = PedersenCompressedCRHGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type SerialNumberNonceGadget = PedersenCompressedCRHGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type PredicateVerificationKeyHashGadget = PedersenCompressedCRHGadget<EdwardsSW, OuterField, EdwardsSWGadget>;

pub type PRFGadget = Blake2sGadget;
pub type PredicateSNARKGadget = GM17VerifierGadget<InnerPairing, OuterField, PairingGadget>;

pub type MerkleTreeIdealLedger = BasicLedger<Tx, CommitmentMerkleParameters>;
pub type Tx = DPCTransaction<Components>;

pub type InstantiatedDPC = DPC<Components>;
pub type LocalData = DPCLocalData<Components>;
