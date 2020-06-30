use crate::dpc::base_dpc::{
    inner_circuit::InnerCircuit,
    inner_circuit_verifier_input::InnerCircuitVerifierInput,
    outer_circuit::OuterCircuit,
    outer_circuit_verifier_input::OuterCircuitVerifierInput,
    predicate::DPCPredicate,
    predicate_circuit::{PredicateCircuit, PredicateLocalData},
    transaction::DPCTransaction,
    BaseDPCComponents,
    LocalData as DPCLocalData,
    DPC,
};
use snarkos_algorithms::{
    commitment::{Blake2sCommitment, PedersenCompressedCommitment},
    crh::{BoweHopwoodPedersenCompressedCRH, PedersenSize},
    define_merkle_tree_parameters,
    prf::Blake2s,
    signature::SchnorrSignature,
    snark::GM17,
};
use snarkos_curves::{
    bls12_377::{fq::Fq as Bls12_377Fq, fr::Fr as Bls12_377Fr, Bls12_377},
    bw6_761::BW6_761,
    edwards_bls12::{EdwardsAffine, EdwardsProjective as EdwardsBls},
    edwards_sw6::EdwardsProjective as EdwardsSW,
};
use snarkos_gadgets::{
    algorithms::{
        binding_signature::BindingSignatureVerificationGadget,
        commitment::{Blake2sCommitmentGadget, PedersenCompressedCommitmentGadget},
        crh::BoweHopwoodPedersenCompressedCRHGadget,
        prf::Blake2sGadget,
        signature::SchnorrPublicKeyRandomizationGadget,
        snark::GM17VerifierGadget,
    },
    curves::{bls12_377::PairingGadget, edwards_bls12::EdwardsBlsGadget, edwards_sw6::EdwardsSWGadget},
};
use snarkos_models::dpc::DPCComponents;

use blake2::Blake2s as Blake2sHash;

pub const NUM_INPUT_RECORDS: usize = 2;
pub const NUM_OUTPUT_RECORDS: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SnNonceWindow;

impl PedersenSize for SnNonceWindow {
    const NUM_WINDOWS: usize = 32;
    const WINDOW_SIZE: usize = 63;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PredVkHashWindow;

impl PedersenSize for PredVkHashWindow {
    const NUM_WINDOWS: usize = 144;
    const WINDOW_SIZE: usize = 63;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocalDataCRHWindow;

impl PedersenSize for LocalDataCRHWindow {
    const NUM_WINDOWS: usize = 16;
    const WINDOW_SIZE: usize = 32;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocalDataCommitmentWindow;

impl PedersenSize for LocalDataCommitmentWindow {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 129;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TwoToOneWindow;
impl PedersenSize for TwoToOneWindow {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 32;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecordWindow;
impl PedersenSize for RecordWindow {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 225;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AccountWindow;
impl PedersenSize for AccountWindow {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 192;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ValueWindow;

impl PedersenSize for ValueWindow {
    const NUM_WINDOWS: usize = 4;
    const WINDOW_SIZE: usize = 350;
}

define_merkle_tree_parameters!(CommitmentMerkleParameters, MerkleTreeCRH, 32);

pub struct Components;

impl DPCComponents for Components {
    type AccountCommitment = AccountCommitment;
    type AccountCommitmentGadget = AccountCommitmentGadget;
    type AccountSignature = AccountSignature;
    type AccountSignatureGadget = AccountSignatureGadget;
    type InnerField = InnerField;
    type LocalDataCRH = LocalDataCRH;
    type LocalDataCRHGadget = LocalDataCRHGadget;
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
    type SerialNumberNonceCRH = SerialNumberNonce;
    type SerialNumberNonceCRHGadget = SerialNumberNonceGadget;

    const NUM_INPUT_RECORDS: usize = NUM_INPUT_RECORDS;
    const NUM_OUTPUT_RECORDS: usize = NUM_OUTPUT_RECORDS;
}

impl BaseDPCComponents for Components {
    type BindingSignatureGadget = BindingSignatureGadget;
    type BindingSignatureGroup = EdwardsBls;
    type InnerSNARK = CoreCheckNIZK;
    type InnerSNARKGadget = InnerSNARKGadget;
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
pub type OuterPairing = BW6_761;
pub type InnerField = Bls12_377Fr;
pub type OuterField = Bls12_377Fq;

pub type AccountCommitment = PedersenCompressedCommitment<EdwardsBls, AccountWindow>;
pub type RecordCommitment = PedersenCompressedCommitment<EdwardsBls, RecordWindow>;
pub type PredicateVerificationKeyCommitment = Blake2sCommitment;
pub type LocalDataCRH = BoweHopwoodPedersenCompressedCRH<EdwardsBls, LocalDataCRHWindow>;
pub type LocalDataCommitment = PedersenCompressedCommitment<EdwardsBls, LocalDataCommitmentWindow>;
pub type ValueCommitment = PedersenCompressedCommitment<EdwardsBls, ValueWindow>;

pub type AccountSignature = SchnorrSignature<EdwardsAffine, Blake2sHash>;

pub type MerkleTreeCRH = BoweHopwoodPedersenCompressedCRH<EdwardsBls, TwoToOneWindow>;
pub type SerialNumberNonce = BoweHopwoodPedersenCompressedCRH<EdwardsBls, SnNonceWindow>;
pub type PredicateVerificationKeyHash = BoweHopwoodPedersenCompressedCRH<EdwardsSW, PredVkHashWindow>;

pub type Predicate = DPCPredicate<Components>;
pub type CoreCheckNIZK = GM17<InnerPairing, InnerCircuit<Components>, InnerCircuitVerifierInput<Components>>;
pub type ProofCheckNIZK = GM17<OuterPairing, OuterCircuit<Components>, OuterCircuitVerifierInput<Components>>;
pub type PredicateSNARK<C> = GM17<InnerPairing, PredicateCircuit<C>, PredicateLocalData<C>>;
pub type PRF = Blake2s;

pub type Tx = DPCTransaction<Components>;

pub type InstantiatedDPC = DPC<Components>;
pub type LocalData = DPCLocalData<Components>;

// Gadgets

pub type AccountCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type RecordCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type PredicateVerificationKeyCommitmentGadget = Blake2sCommitmentGadget;
pub type LocalDataCRHGadget = BoweHopwoodPedersenCompressedCRHGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type LocalDataCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type ValueCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;

pub type BindingSignatureGadget = BindingSignatureVerificationGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type AccountSignatureGadget = SchnorrPublicKeyRandomizationGadget<EdwardsAffine, InnerField, EdwardsBlsGadget>;

pub type MerkleTreeCRHGadget = BoweHopwoodPedersenCompressedCRHGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type SerialNumberNonceGadget = BoweHopwoodPedersenCompressedCRHGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type PredicateVerificationKeyHashGadget =
    BoweHopwoodPedersenCompressedCRHGadget<EdwardsSW, OuterField, EdwardsSWGadget>;

pub type PRFGadget = Blake2sGadget;
pub type PredicateSNARKGadget = GM17VerifierGadget<InnerPairing, OuterField, PairingGadget>;
pub type InnerSNARKGadget = GM17VerifierGadget<InnerPairing, OuterField, PairingGadget>;
