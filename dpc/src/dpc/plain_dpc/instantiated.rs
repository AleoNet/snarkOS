use crate::{
    dpc::plain_dpc::{
        core_checks_circuit::*, predicate::DPCPredicate, proof_check_circuit::*,
        transaction::DPCTransaction, LocalData as DPCLocalData, PlainDPCComponents, DPC,
        predicate_circuit::{EmptyPredicateCircuit, PredicateLocalData},
    },
    ledger::ideal_ledger::IdealLedger,
};
use snarkos_algorithms::{
    commitment::{Blake2sCommitment, PedersenCompressedCommitment},
    crh::{PedersenCompressedCRH, PedersenSize},
    merkle_tree::MerkleParameters,
    prf::Blake2s,
    snark::GM17,
};
use snarkos_curves::{
    bls12_377::{fq::Fq as Bls12_377Fq, fr::Fr as Bls12_377Fr, Bls12_377},
    edwards_bls12::EdwardsProjective as EdwardsBls,
    edwards_sw6::EdwardsProjective as EdwardsSW,
    sw6::SW6,
};
use snarkos_gadgets::{
    algorithms::{
        commitment::{Blake2sCommitmentGadget, PedersenCompressedCommitmentGadget},
        crh::PedersenCompressedCRHGadget,
        prf::Blake2sGadget,
        snark::GM17VerifierGadget,
    },
    curves::{
        bls12_377::PairingGadget, edwards_bls12::EdwardsBlsGadget, edwards_sw6::EdwardsSWGadget,
    },
};
use snarkos_models::algorithms::CRH;

pub const NUM_INPUT_RECORDS: usize = 2;
pub const NUM_OUTPUT_RECORDS: usize = 2;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SnNonceWindow;

// `WINDOW_SIZE * NUM_WINDOWS` = 2 * 256 + 8 + 256 bits
const SN_NONCE_SIZE_BITS: usize = NUM_INPUT_RECORDS * 2 * 256 + 8 + 256;
impl PedersenSize for SnNonceWindow {
    const WINDOW_SIZE: usize = SN_NONCE_SIZE_BITS / 8;
    const NUM_WINDOWS: usize = 8;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PredVkHashWindow;

impl PedersenSize for PredVkHashWindow {
    const WINDOW_SIZE: usize = 248;
    const NUM_WINDOWS: usize = 38;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LocalDataWindow;

impl PedersenSize for LocalDataWindow {
    const WINDOW_SIZE: usize = 248;
    const NUM_WINDOWS: usize = 30;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TwoToOneWindow;
// `WINDOW_SIZE * NUM_WINDOWS` = 2 * 256 bits
impl PedersenSize for TwoToOneWindow {
    const WINDOW_SIZE: usize = 128;
    const NUM_WINDOWS: usize = 4;
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

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RecordWindow;
impl PedersenSize for RecordWindow {
    const WINDOW_SIZE: usize = 225;
    const NUM_WINDOWS: usize = 8;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AddressWindow;
impl PedersenSize for AddressWindow {
    const WINDOW_SIZE: usize = 128;
    const NUM_WINDOWS: usize = 4;
}

pub struct Components;

impl PlainDPCComponents for Components {
    const NUM_INPUT_RECORDS: usize = NUM_INPUT_RECORDS;
    const NUM_OUTPUT_RECORDS: usize = NUM_OUTPUT_RECORDS;

    type CoreCheckF = CoreCheckF;
    type ProofCheckF = ProofCheckF;

    type MerkleParameters = CommitmentMerkleParameters;
    type MerkleTree_HGadget = MerkleTreeCRHGadget;

    type AddrC = AddressComm;
    type RecC = RecordComm;

    type AddrCGadget = AddressCommGadget;
    type RecCGadget = RecordCommGadget;

    type SnNonceH = SnNonceCRH;
    type SnNonceHGadget = SnNonceCRHGadget;
    type MainNIZK = CoreCheckNIZK;
    type ProofCheckNIZK = ProofCheckNIZK;
    type P = PRF;
    type PGadget = PRFGadget;

    type PredicateNIZK = PredicateNIZK<Self>;
    type PredicateNIZKGadget = PredicateNIZKGadget;

    type PredVkH = PredVkCRH;
    type PredVkHGadget = PredVkCRHGadget;
    type PredVkComm = PredicateComm;
    type PredVkCommGadget = PredicateCommGadget;
    type LocalDataComm = LocalDataComm;
    type LocalDataCommGadget = LocalDataCommGadget;
}

// Native primitives

pub type CoreCheckPairing = Bls12_377;
pub type ProofCheckPairing = SW6;
pub type CoreCheckF = Bls12_377Fr;
pub type ProofCheckF = Bls12_377Fq;

pub type AddressComm = PedersenCompressedCommitment<EdwardsBls, AddressWindow>;
pub type RecordComm = PedersenCompressedCommitment<EdwardsBls, RecordWindow>;
pub type PredicateComm = Blake2sCommitment;
pub type LocalDataComm = PedersenCompressedCommitment<EdwardsBls, LocalDataWindow>;

pub type MerkleTreeCRH = PedersenCompressedCRH<EdwardsBls, TwoToOneWindow>;
pub type SnNonceCRH = PedersenCompressedCRH<EdwardsBls, SnNonceWindow>;
pub type PredVkCRH = PedersenCompressedCRH<EdwardsSW, PredVkHashWindow>;

pub type Predicate = DPCPredicate<Components>;
pub type CoreCheckNIZK =
    GM17<CoreCheckPairing, CoreChecksCircuit<Components>, CoreChecksVerifierInput<Components>>;
pub type ProofCheckNIZK =
    GM17<ProofCheckPairing, ProofCheckCircuit<Components>, ProofCheckVerifierInput<Components>>;
pub type PredicateNIZK<C> = GM17<CoreCheckPairing, EmptyPredicateCircuit<C>, PredicateLocalData<C>>;
pub type PRF = Blake2s;

// Gadgets

pub type RecordCommGadget = PedersenCompressedCommitmentGadget<EdwardsBls, CoreCheckF, EdwardsBlsGadget>;
pub type AddressCommGadget = PedersenCompressedCommitmentGadget<EdwardsBls, CoreCheckF, EdwardsBlsGadget>;
pub type PredicateCommGadget = Blake2sCommitmentGadget;
pub type LocalDataCommGadget = PedersenCompressedCommitmentGadget<EdwardsBls, CoreCheckF, EdwardsBlsGadget>;

pub type SnNonceCRHGadget = PedersenCompressedCRHGadget<EdwardsBls, CoreCheckF, EdwardsBlsGadget>;
pub type MerkleTreeCRHGadget = PedersenCompressedCRHGadget<EdwardsBls, CoreCheckF, EdwardsBlsGadget>;
pub type PredVkCRHGadget = PedersenCompressedCRHGadget<EdwardsSW, ProofCheckF, EdwardsSWGadget>;

pub type PRFGadget = Blake2sGadget;
pub type PredicateNIZKGadget = GM17VerifierGadget<CoreCheckPairing, ProofCheckF, PairingGadget>;


pub type MerkleTreeIdealLedger = IdealLedger<Tx, CommitmentMerkleParameters>;
pub type Tx = DPCTransaction<Components>;

pub type InstantiatedDPC = DPC<Components>;
pub type LocalData = DPCLocalData<Components>;
