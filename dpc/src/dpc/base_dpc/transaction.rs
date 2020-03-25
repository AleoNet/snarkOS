use crate::{dpc::base_dpc::BaseDPCComponents, Transaction};
use snarkos_algorithms::merkle_tree::MerkleTreeDigest;
use snarkos_models::algorithms::{CommitmentScheme, SignatureScheme, SNARK};

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: BaseDPCComponents"),
    PartialEq(bound = "C: BaseDPCComponents"),
    Eq(bound = "C: BaseDPCComponents")
)]
pub struct DPCTransaction<C: BaseDPCComponents> {
    old_serial_numbers: Vec<<C::Signature as SignatureScheme>::PublicKey>,
    new_commitments: Vec<<C::RecordCommitment as CommitmentScheme>::Output>,
    memorandum: [u8; 32],
    pub stuff: DPCStuff<C>,
}

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: BaseDPCComponents"),
    PartialEq(bound = "C: BaseDPCComponents"),
    Eq(bound = "C: BaseDPCComponents")
)]
pub struct DPCStuff<C: BaseDPCComponents> {
    pub digest: MerkleTreeDigest<C::MerkleParameters>,
    #[derivative(PartialEq = "ignore")]
    pub inner_proof: <C::InnerSNARK as SNARK>::Proof,
    #[derivative(PartialEq = "ignore")]
    pub predicate_proof: <C::OuterSNARK as SNARK>::Proof,
    #[derivative(PartialEq = "ignore")]
    pub predicate_commitment: <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    #[derivative(PartialEq = "ignore")]
    pub local_data_commitment: <C::LocalDataCommitment as CommitmentScheme>::Output,

    pub value_balance: u64,

    #[derivative(PartialEq = "ignore")]
    pub signatures: Vec<<C::Signature as SignatureScheme>::Output>,
}

impl<C: BaseDPCComponents> DPCTransaction<C> {
    pub fn new(
        old_serial_numbers: Vec<<Self as Transaction>::SerialNumber>,
        new_commitments: Vec<<Self as Transaction>::Commitment>,
        memorandum: <Self as Transaction>::Memorandum,
        digest: MerkleTreeDigest<C::MerkleParameters>,
        inner_proof: <C::InnerSNARK as SNARK>::Proof,
        predicate_proof: <C::OuterSNARK as SNARK>::Proof,
        predicate_commitment: <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
        local_data_commitment: <C::LocalDataCommitment as CommitmentScheme>::Output,
        value_balance: u64,
        signatures: Vec<<C::Signature as SignatureScheme>::Output>,
    ) -> Self {
        let stuff = DPCStuff {
            digest,
            inner_proof,
            predicate_proof,
            predicate_commitment,
            local_data_commitment,
            value_balance,
            signatures,
        };
        DPCTransaction {
            old_serial_numbers,
            new_commitments,
            memorandum,
            stuff,
        }
    }
}

impl<C: BaseDPCComponents> Transaction for DPCTransaction<C> {
    type Commitment = <C::RecordCommitment as CommitmentScheme>::Output;
    type Memorandum = [u8; 32];
    type SerialNumber = <C::Signature as SignatureScheme>::PublicKey;
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
