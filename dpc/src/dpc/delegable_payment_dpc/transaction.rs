use crate::{
    dpc::delegable_payment_dpc::{binding_signature::BindingSignature, DelegablePaymentDPCComponents},
    Transaction,
};
use snarkos_algorithms::merkle_tree::MerkleTreeDigest;
use snarkos_models::algorithms::{CommitmentScheme, SignatureScheme, SNARK};

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: DelegablePaymentDPCComponents"),
    PartialEq(bound = "C: DelegablePaymentDPCComponents"),
    Eq(bound = "C: DelegablePaymentDPCComponents")
)]
pub struct DPCTransaction<C: DelegablePaymentDPCComponents> {
    old_serial_numbers: Vec<<C::S as SignatureScheme>::PublicKey>,
    new_commitments: Vec<<C::RecC as CommitmentScheme>::Output>,
    memorandum: [u8; 32],
    pub stuff: DPCStuff<C>,
}

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: DelegablePaymentDPCComponents"),
    PartialEq(bound = "C: DelegablePaymentDPCComponents"),
    Eq(bound = "C: DelegablePaymentDPCComponents")
)]
pub struct DPCStuff<C: DelegablePaymentDPCComponents> {
    pub digest: MerkleTreeDigest<C::MerkleParameters>,
    #[derivative(PartialEq = "ignore")]
    pub core_proof: <C::MainNIZK as SNARK>::Proof,
    #[derivative(PartialEq = "ignore")]
    pub predicate_proof: <C::ProofCheckNIZK as SNARK>::Proof,
    #[derivative(PartialEq = "ignore")]
    pub predicate_comm: <C::PredVkComm as CommitmentScheme>::Output,
    #[derivative(PartialEq = "ignore")]
    pub local_data_comm: <C::LocalDataComm as CommitmentScheme>::Output,

    pub input_value_commitments: Vec<[u8; 32]>,
    pub output_value_commitments: Vec<[u8; 32]>,
    pub value_balance: u64,
    pub binding_signature: BindingSignature,

    #[derivative(PartialEq = "ignore")]
    pub signatures: Vec<<C::S as SignatureScheme>::Output>,
}

impl<C: DelegablePaymentDPCComponents> DPCTransaction<C> {
    pub fn new(
        old_serial_numbers: Vec<<Self as Transaction>::SerialNumber>,
        new_commitments: Vec<<Self as Transaction>::Commitment>,
        memorandum: <Self as Transaction>::Memorandum,
        digest: MerkleTreeDigest<C::MerkleParameters>,
        core_proof: <C::MainNIZK as SNARK>::Proof,
        predicate_proof: <C::ProofCheckNIZK as SNARK>::Proof,
        predicate_comm: <C::PredVkComm as CommitmentScheme>::Output,
        local_data_comm: <C::LocalDataComm as CommitmentScheme>::Output,
        input_value_commitments: Vec<[u8; 32]>,
        output_value_commitments: Vec<[u8; 32]>,
        value_balance: u64,
        binding_signature: BindingSignature,
        signatures: Vec<<C::S as SignatureScheme>::Output>,
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

impl<C: DelegablePaymentDPCComponents> Transaction for DPCTransaction<C> {
    type Commitment = <C::RecC as CommitmentScheme>::Output;
    type Memorandum = [u8; 32];
    type SerialNumber = <C::S as SignatureScheme>::PublicKey;
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
