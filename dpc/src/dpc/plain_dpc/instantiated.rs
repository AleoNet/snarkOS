use crate::{
    dpc::plain_dpc::{
        core_checks_circuit::*,
        predicate::DPCPredicate,
        predicate_circuit::{EmptyPredicateCircuit, PredicateLocalData},
        proof_check_circuit::*,
        transaction::DPCTransaction,
        DPCComponents,
        LocalData as DPCLocalData,
        PlainDPCComponents,
        DPC,
    },
    ledger::ideal_ledger::IdealLedger,
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
        commitment::{Blake2sCommitmentGadget, PedersenCompressedCommitmentGadget},
        crh::PedersenCompressedCRHGadget,
        prf::Blake2sGadget,
        signature::SchnorrPublicKeyRandomizationGadget,
        snark::GM17VerifierGadget,
    },
    curves::{bls12_377::PairingGadget, edwards_bls12::EdwardsBlsGadget, edwards_sw6::EdwardsSWGadget},
};
use snarkos_models::algorithms::CRH;

use blake2::Blake2s as Blake2sHash;

pub const NUM_INPUT_RECORDS: usize = 2;
pub const NUM_OUTPUT_RECORDS: usize = 2;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SnNonceWindow;

// `WINDOW_SIZE * NUM_WINDOWS` = 2 * 256 + 8 + 256 bits
const SN_NONCE_SIZE_BITS: usize = NUM_INPUT_RECORDS * 2 * 256 + 8 + 256;
impl PedersenSize for SnNonceWindow {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = SN_NONCE_SIZE_BITS / 8;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PredVkHashWindow;

impl PedersenSize for PredVkHashWindow {
    const NUM_WINDOWS: usize = 38;
    const WINDOW_SIZE: usize = 248;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LocalDataWindow;

impl PedersenSize for LocalDataWindow {
    const NUM_WINDOWS: usize = 30;
    const WINDOW_SIZE: usize = 248;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TwoToOneWindow;
// `WINDOW_SIZE * NUM_WINDOWS` = 2 * 256 bits
impl PedersenSize for TwoToOneWindow {
    const NUM_WINDOWS: usize = 4;
    const WINDOW_SIZE: usize = 128;
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
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 225;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AddressWindow;
impl PedersenSize for AddressWindow {
    const NUM_WINDOWS: usize = 4;
    const WINDOW_SIZE: usize = 128;
}

pub struct Components;

impl PlainDPCComponents for Components {
    type MainNIZK = CoreCheckNIZK;
    type PredicateNIZK = PredicateNIZK<Self>;
    type PredicateNIZKGadget = PredicateNIZKGadget;
    type ProofCheckNIZK = ProofCheckNIZK;
}

impl DPCComponents for Components {
    type AddressCommitment = AddressCommitment;
    type AddressCommitmentGadget = AddressCommitmentGadget;
    type InnerField = InnerField;
    type LocalDataCommitment = LocalDataCommitment;
    type LocalDataCommitmentGadget = LocalDataCommitmentGadget;
    type MerkleHashGadget = MerkleHashGadget;
    type MerkleParameters = CommitmentMerkleParameters;
    type OuterField = OuterField;
    type P = PRF;
    type PGadget = PRFGadget;
    type PredicateVerificationKeyCommitment = PredicateComm;
    type PredicateVerificationKeyCommitmentGadget = PredicateCommGadget;
    type PredicateVerificationKeyHash = PredVkCRH;
    type PredicateVerificationKeyHashGadget = PredVkCRHGadget;
    type RecordCommitment = RecordCommitment;
    type RecordCommitmentGadget = RecordCommitmentGadget;
    type SerialNumberNonce = SnNonceCRH;
    type SerialNumberNonceGadget = SnNonceCRHGadget;
    type Signature = AuthSignature;
    type SignatureGadget = AuthSignatureGadget;

    const NUM_INPUT_RECORDS: usize = NUM_INPUT_RECORDS;
    const NUM_OUTPUT_RECORDS: usize = NUM_OUTPUT_RECORDS;
}

pub type InnerField = Bls12_377Fr;
pub type OuterField = Bls12_377Fq;

pub type InnerPairing = Bls12_377;
pub type OuterPairing = SW6;

pub type AddressCommitment = PedersenCompressedCommitment<EdwardsBls, AddressWindow>;
pub type AddressCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;

pub type RecordCommitment = PedersenCompressedCommitment<EdwardsBls, RecordWindow>;
pub type RecordCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;

pub type LocalDataCommitment = PedersenCompressedCommitment<EdwardsBls, LocalDataWindow>;
pub type LocalDataCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;

pub type PRF = Blake2s;
pub type PredicateComm = Blake2sCommitment;

pub type AuthSignature = SchnorrSignature<EdwardsAffine, Blake2sHash>;

pub type MerkleTreeCRH = PedersenCompressedCRH<EdwardsBls, TwoToOneWindow>;
pub type SnNonceCRH = PedersenCompressedCRH<EdwardsBls, SnNonceWindow>;
pub type PredVkCRH = PedersenCompressedCRH<EdwardsSW, PredVkHashWindow>;

pub type Predicate = DPCPredicate<Components>;
pub type CoreCheckNIZK = GM17<InnerPairing, CoreChecksCircuit<Components>, CoreChecksVerifierInput<Components>>;
pub type ProofCheckNIZK = GM17<OuterPairing, ProofCheckCircuit<Components>, ProofCheckVerifierInput<Components>>;
pub type PredicateNIZK<C> = GM17<InnerPairing, EmptyPredicateCircuit<C>, PredicateLocalData<C>>;

// Gadgets

pub type PredicateCommGadget = Blake2sCommitmentGadget;

pub type SnNonceCRHGadget = PedersenCompressedCRHGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type MerkleHashGadget = PedersenCompressedCRHGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type PredVkCRHGadget = PedersenCompressedCRHGadget<EdwardsSW, OuterField, EdwardsSWGadget>;

pub type PRFGadget = Blake2sGadget;
pub type PredicateNIZKGadget = GM17VerifierGadget<InnerPairing, OuterField, PairingGadget>;

pub type AuthSignatureGadget = SchnorrPublicKeyRandomizationGadget<EdwardsAffine, InnerField, EdwardsBlsGadget>;

pub type MerkleTreeIdealLedger = IdealLedger<Tx, CommitmentMerkleParameters>;
pub type Tx = DPCTransaction<Components>;

pub type InstantiatedDPC = DPC<Components>;
pub type LocalData = DPCLocalData<Components>;
