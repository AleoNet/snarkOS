use crate::base_dpc::{
    inner_circuit::InnerCircuit,
    inner_circuit_verifier_input::InnerCircuitVerifierInput,
    outer_circuit::OuterCircuit,
    outer_circuit_verifier_input::OuterCircuitVerifierInput,
    program::{NoopCircuit, ProgramLocalData},
    transaction::DPCTransaction,
    BaseDPCComponents,
    LocalData as DPCLocalData,
    DPC,
};
use snarkos_algorithms::{
    commitment::{Blake2sCommitment, PedersenCompressedCommitment},
    crh::{BoweHopwoodPedersenCompressedCRH, PedersenSize},
    define_merkle_tree_parameters,
    encryption::GroupEncryption,
    prf::Blake2s,
    signature::SchnorrSignature,
    snark::{gm17::GM17, groth16::Groth16},
};
use snarkos_curves::{
    bls12_377::{fq::Fq as Bls12_377Fq, fr::Fr as Bls12_377Fr, Bls12_377},
    bw6_761::BW6_761,
    edwards_bls12::{EdwardsAffine, EdwardsParameters, EdwardsProjective as EdwardsBls},
    edwards_sw6::EdwardsProjective as EdwardsSW,
};
use snarkos_gadgets::{
    algorithms::{
        commitment::{Blake2sCommitmentGadget, PedersenCompressedCommitmentGadget},
        crh::BoweHopwoodPedersenCompressedCRHGadget,
        encryption::GroupEncryptionGadget,
        prf::Blake2sGadget,
        signature::SchnorrPublicKeyRandomizationGadget,
        snark::{GM17VerifierGadget, Groth16VerifierGadget},
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
pub struct ProgramVkHashWindow;

impl PedersenSize for ProgramVkHashWindow {
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
    const WINDOW_SIZE: usize = 233;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EncryptedRecordWindow;

impl PedersenSize for EncryptedRecordWindow {
    const NUM_WINDOWS: usize = 48;
    const WINDOW_SIZE: usize = 44;
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
    type AccountEncryption = AccountEncryption;
    type AccountEncryptionGadget = AccountEncryptionGadget;
    type AccountSignature = AccountSignature;
    type AccountSignatureGadget = AccountSignatureGadget;
    type EncryptedRecordCRH = EncryptedRecordCRH;
    type EncryptedRecordCRHGadget = EncryptedRecordCRHGadget;
    type InnerField = InnerField;
    type LocalDataCRH = LocalDataCRH;
    type LocalDataCRHGadget = LocalDataCRHGadget;
    type LocalDataCommitment = LocalDataCommitment;
    type LocalDataCommitmentGadget = LocalDataCommitmentGadget;
    type OuterField = OuterField;
    type PRF = PRF;
    type PRFGadget = PRFGadget;
    type ProgramVerificationKeyCRH = ProgramVerificationKeyCRH;
    type ProgramVerificationKeyCRHGadget = ProgramVerificationKeyCRHGadget;
    type ProgramVerificationKeyCommitment = ProgramVerificationKeyCommitment;
    type ProgramVerificationKeyCommitmentGadget = ProgramVerificationKeyCommitmentGadget;
    type RecordCommitment = RecordCommitment;
    type RecordCommitmentGadget = RecordCommitmentGadget;
    type SerialNumberNonceCRH = SerialNumberNonce;
    type SerialNumberNonceCRHGadget = SerialNumberNonceGadget;

    const NUM_INPUT_RECORDS: usize = NUM_INPUT_RECORDS;
    const NUM_OUTPUT_RECORDS: usize = NUM_OUTPUT_RECORDS;
}

impl BaseDPCComponents for Components {
    type EncryptionGroup = EdwardsBls;
    type EncryptionModelParameters = EdwardsParameters;
    type InnerSNARK = CoreCheckNIZK;
    type InnerSNARKGadget = InnerSNARKGadget;
    type MerkleHashGadget = MerkleTreeCRHGadget;
    type MerkleParameters = CommitmentMerkleParameters;
    type NoopProgramSNARK = NoopProgramSNARK<Self>;
    type OuterSNARK = ProofCheckNIZK;
    type ProgramSNARKGadget = ProgramSNARKGadget;
}

// Native primitives

pub type InnerPairing = Bls12_377;
pub type OuterPairing = BW6_761;
pub type InnerField = Bls12_377Fr;
pub type OuterField = Bls12_377Fq;

pub type AccountCommitment = PedersenCompressedCommitment<EdwardsBls, AccountWindow>;
pub type AccountEncryption = GroupEncryption<EdwardsBls>;
pub type RecordCommitment = PedersenCompressedCommitment<EdwardsBls, RecordWindow>;
pub type ProgramVerificationKeyCommitment = Blake2sCommitment;
pub type LocalDataCRH = BoweHopwoodPedersenCompressedCRH<EdwardsBls, LocalDataCRHWindow>;
pub type LocalDataCommitment = PedersenCompressedCommitment<EdwardsBls, LocalDataCommitmentWindow>;

pub type AccountSignature = SchnorrSignature<EdwardsAffine, Blake2sHash>;

pub type MerkleTreeCRH = BoweHopwoodPedersenCompressedCRH<EdwardsBls, TwoToOneWindow>;
pub type EncryptedRecordCRH = BoweHopwoodPedersenCompressedCRH<EdwardsBls, EncryptedRecordWindow>;
pub type SerialNumberNonce = BoweHopwoodPedersenCompressedCRH<EdwardsBls, SnNonceWindow>;
pub type ProgramVerificationKeyCRH = BoweHopwoodPedersenCompressedCRH<EdwardsSW, ProgramVkHashWindow>;

pub type CoreCheckNIZK = Groth16<InnerPairing, InnerCircuit<Components>, InnerCircuitVerifierInput<Components>>;
pub type ProofCheckNIZK = Groth16<OuterPairing, OuterCircuit<Components>, OuterCircuitVerifierInput<Components>>;
pub type NoopProgramSNARK<C> = GM17<InnerPairing, NoopCircuit<C>, ProgramLocalData<C>>;
pub type PRF = Blake2s;

pub type Tx = DPCTransaction<Components>;

pub type InstantiatedDPC = DPC<Components>;
pub type LocalData = DPCLocalData<Components>;

// Gadgets

pub type AccountCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type AccountEncryptionGadget = GroupEncryptionGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type RecordCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type ProgramVerificationKeyCommitmentGadget = Blake2sCommitmentGadget;
pub type LocalDataCRHGadget = BoweHopwoodPedersenCompressedCRHGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type LocalDataCommitmentGadget = PedersenCompressedCommitmentGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;

pub type AccountSignatureGadget = SchnorrPublicKeyRandomizationGadget<EdwardsAffine, InnerField, EdwardsBlsGadget>;

pub type MerkleTreeCRHGadget = BoweHopwoodPedersenCompressedCRHGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type EncryptedRecordCRHGadget = BoweHopwoodPedersenCompressedCRHGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type SerialNumberNonceGadget = BoweHopwoodPedersenCompressedCRHGadget<EdwardsBls, InnerField, EdwardsBlsGadget>;
pub type ProgramVerificationKeyCRHGadget =
    BoweHopwoodPedersenCompressedCRHGadget<EdwardsSW, OuterField, EdwardsSWGadget>;

pub type PRFGadget = Blake2sGadget;
pub type ProgramSNARKGadget = GM17VerifierGadget<InnerPairing, OuterField, PairingGadget>;
pub type InnerSNARKGadget = Groth16VerifierGadget<InnerPairing, OuterField, PairingGadget>;
