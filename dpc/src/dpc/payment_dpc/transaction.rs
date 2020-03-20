use crate::{
    dpc::payment_dpc::{binding_signature::BindingSignature, PaymentDPCComponents},
    Transaction,
};
use snarkos_algorithms::merkle_tree::MerkleTreeDigest;
use snarkos_models::algorithms::{CommitmentScheme, PRF, SNARK};

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: PaymentDPCComponents"),
    PartialEq(bound = "C: PaymentDPCComponents"),
    Eq(bound = "C: PaymentDPCComponents")
)]
pub struct DPCTransaction<C: PaymentDPCComponents> {
    old_serial_numbers: Vec<<C::P as PRF>::Output>,
    new_commitments: Vec<<C::RecordCommitment as CommitmentScheme>::Output>,
    memorandum: [u8; 32],
    pub stuff: DPCStuff<C>,
}

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: PaymentDPCComponents"),
    PartialEq(bound = "C: PaymentDPCComponents"),
    Eq(bound = "C: PaymentDPCComponents")
)]
pub struct DPCStuff<C: PaymentDPCComponents> {
    pub digest: MerkleTreeDigest<C::MerkleParameters>,
    #[derivative(PartialEq = "ignore")]
    pub core_proof: <C::MainNIZK as SNARK>::Proof,
    #[derivative(PartialEq = "ignore")]
    pub predicate_proof: <C::ProofCheckNIZK as SNARK>::Proof,
    #[derivative(PartialEq = "ignore")]
    pub predicate_comm: <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    #[derivative(PartialEq = "ignore")]
    pub local_data_comm: <C::LocalDataCommitment as CommitmentScheme>::Output,

    pub input_value_commitments: Vec<[u8; 32]>,
    pub output_value_commitments: Vec<[u8; 32]>,
    pub value_balance: u64,
    pub binding_signature: BindingSignature,
}

impl<C: PaymentDPCComponents> DPCTransaction<C> {
    pub fn new(
        old_serial_numbers: Vec<<Self as Transaction>::SerialNumber>,
        new_commitments: Vec<<Self as Transaction>::Commitment>,
        memorandum: <Self as Transaction>::Memorandum,
        digest: MerkleTreeDigest<C::MerkleParameters>,
        core_proof: <C::MainNIZK as SNARK>::Proof,
        predicate_proof: <C::ProofCheckNIZK as SNARK>::Proof,
        predicate_comm: <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
        local_data_comm: <C::LocalDataCommitment as CommitmentScheme>::Output,
        input_value_commitments: Vec<[u8; 32]>,
        output_value_commitments: Vec<[u8; 32]>,
        value_balance: u64,
        binding_signature: BindingSignature,
    ) -> Self {
        let stuff = DPCStuff {
            digest,
            core_proof,
            predicate_proof,
            predicate_comm,
            local_data_comm,
            input_value_commitments,
            output_value_commitments,
            value_balance,
            binding_signature,
        };
        DPCTransaction {
            old_serial_numbers,
            new_commitments,
            memorandum,
            stuff,
        }
    }
}

impl<C: PaymentDPCComponents> Transaction for DPCTransaction<C> {
    type Commitment = <C::RecordCommitment as CommitmentScheme>::Output;
    type Memorandum = [u8; 32];
    type SerialNumber = <C::P as PRF>::Output;
    type Stuff = DPCStuff<C>;

    fn old_serial_numbers(&self) -> &[Self::SerialNumber] {
        self.old_serial_numbers.as_slice()
    }

    fn new_commitments(&self) -> &[Self::Commitment] {
        self.new_commitments.as_slice()
    }

    fn memorandum(&self) -> &Self::Memorandum {
        &self.memorandum
    }

    fn stuff(&self) -> &Self::Stuff {
        &self.stuff
    }
}
