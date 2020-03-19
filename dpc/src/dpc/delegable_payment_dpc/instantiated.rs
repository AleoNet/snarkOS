use crate::{
    dpc::{
        delegable_payment_dpc::{
            core_checks_circuit::*,
            payment_circuit::{PaymentCircuit, PaymentPredicateLocalData},
            predicate::DPCPredicate,
            proof_check_circuit::*,
            transaction::DPCTransaction,
            DelegablePaymentDPCComponents,
            LocalData as DPCLocalData,
            DPC,
        },
        plain_dpc::DPCComponents,
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

impl DelegablePaymentDPCComponents for Components {
    type MainNIZK = CoreCheckNIZK;
    type PredicateNIZK = PredicateNIZK<Self>;
    type PredicateNIZKGadget = PredicateNIZKGadget;
    type ProofCheckNIZK = ProofCheckNIZK;
    type ValueComm = ValueComm;
    type ValueCommGadget = ValueCommGadget;
}

impl DPCComponents for Components {
    type AddrC = AddressComm;
    type AddrCGadget = AddressCommGadget;
    type InnerField = CoreCheckF;
    type LocalDataCommitment = LocalDataComm;
    type LocalDataCommitmentGadget = LocalDataCommGadget;
    type MerkleParameters = CommitmentMerkleParameters;
    type MerkleTreeHGadget = MerkleTreeCRHGadget;
    type OuterField = ProofCheckF;
    type P = PRF;
    type PGadget = PRFGadget;
    type PredVkComm = PredicateComm;
    type PredVkCommGadget = PredicateCommGadget;
    type PredVkH = PredVkCRH;
    type PredVkHGadget = PredVkCRHGadget;
    type RecC = RecordComm;
    type RecCGadget = RecordCommGadget;
    type Signature = AuthSignature;
    type SignatureGadget = AuthSignatureGadget;
    type SnNonceH = SnNonceCRH;
    type SnNonceHGadget = SnNonceCRHGadget;

    const NUM_INPUT_RECORDS: usize = NUM_INPUT_RECORDS;
    const NUM_OUTPUT_RECORDS: usize = NUM_OUTPUT_RECORDS;
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
pub type ValueComm = PedersenCompressedCommitment<EdwardsBls, ValueWindow>;

pub type AuthSignature = SchnorrSignature<EdwardsAffine, Blake2sHash>;

pub type MerkleTreeCRH = PedersenCompressedCRH<EdwardsBls, TwoToOneWindow>;
pub type SnNonceCRH = PedersenCompressedCRH<EdwardsBls, SnNonceWindow>;
pub type PredVkCRH = PedersenCompressedCRH<EdwardsSW, PredVkHashWindow>;

pub type Predicate = DPCPredicate<Components>;
pub type CoreCheckNIZK = GM17<CoreCheckPairing, CoreChecksCircuit<Components>, CoreChecksVerifierInput<Components>>;
pub type ProofCheckNIZK = GM17<ProofCheckPairing, ProofCheckCircuit<Components>, ProofCheckVerifierInput<Components>>;
pub type PredicateNIZK<C> = GM17<CoreCheckPairing, PaymentCircuit<C>, PaymentPredicateLocalData<C>>;
pub type PRF = Blake2s;

// Gadgets

pub type RecordCommGadget = PedersenCompressedCommitmentGadget<EdwardsBls, CoreCheckF, EdwardsBlsGadget>;
pub type AddressCommGadget = PedersenCompressedCommitmentGadget<EdwardsBls, CoreCheckF, EdwardsBlsGadget>;
pub type PredicateCommGadget = Blake2sCommitmentGadget;
pub type LocalDataCommGadget = PedersenCompressedCommitmentGadget<EdwardsBls, CoreCheckF, EdwardsBlsGadget>;
pub type ValueCommGadget = PedersenCompressedCommitmentGadget<EdwardsBls, CoreCheckF, EdwardsBlsGadget>;

pub type SnNonceCRHGadget = PedersenCompressedCRHGadget<EdwardsBls, CoreCheckF, EdwardsBlsGadget>;
pub type MerkleTreeCRHGadget = PedersenCompressedCRHGadget<EdwardsBls, CoreCheckF, EdwardsBlsGadget>;
pub type PredVkCRHGadget = PedersenCompressedCRHGadget<EdwardsSW, ProofCheckF, EdwardsSWGadget>;

pub type PRFGadget = Blake2sGadget;
pub type PredicateNIZKGadget = GM17VerifierGadget<CoreCheckPairing, ProofCheckF, PairingGadget>;

pub type AuthSignatureGadget = SchnorrPublicKeyRandomizationGadget<EdwardsAffine, CoreCheckF, EdwardsBlsGadget>;

pub type MerkleTreeIdealLedger = IdealLedger<Tx, CommitmentMerkleParameters>;
pub type Tx = DPCTransaction<Components>;

pub type InstantiatedDPC = DPC<Components>;
pub type LocalData = DPCLocalData<Components>;
