use crate::dpc::base_dpc::BaseDPCComponents;
use snarkos_algorithms::merkle_tree::MerkleTreeDigest;
use snarkos_errors::objects::TransactionError;
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, SNARK},
    objects::Transaction,
};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    serialize::{CanonicalDeserialize, CanonicalSerialize},
    to_bytes,
    variable_length_integer::{read_variable_length_integer, variable_length_integer},
};

use blake2::{digest::Digest, Blake2s as b2s};
use std::{
    fmt,
    io::{Read, Result as IoResult, Write},
};

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: BaseDPCComponents"),
    PartialEq(bound = "C: BaseDPCComponents"),
    Eq(bound = "C: BaseDPCComponents")
)]
// TODO (howardwu): Remove the public visibility here
pub struct DPCTransaction<C: BaseDPCComponents> {
    /// The network this transaction is included in
    pub network_id: u8,

    pub ledger_digest: MerkleTreeDigest<C::MerkleParameters>,

    pub old_serial_numbers: Vec<<C::AccountSignature as SignatureScheme>::PublicKey>,

    pub new_commitments: Vec<<C::RecordCommitment as CommitmentScheme>::Output>,

    #[derivative(PartialEq = "ignore")]
    pub predicate_commitment: <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,

    #[derivative(PartialEq = "ignore")]
    pub local_data_commitment: <C::LocalDataCommitment as CommitmentScheme>::Output,

    /// A transaction value balance is the difference between input and output record balances.
    /// This value effectively becomes the transaction fee for the miner. Only coinbase transactions
    /// can have a negative value balance representing tokens being minted.
    pub value_balance: i64,

    #[derivative(PartialEq = "ignore")]
    pub signatures: Vec<<C::AccountSignature as SignatureScheme>::Output>,

    #[derivative(PartialEq = "ignore")]
    pub transaction_proof: <C::OuterSNARK as SNARK>::Proof,

    pub memorandum: [u8; 32],
}

impl<C: BaseDPCComponents> DPCTransaction<C> {
    pub fn new(
        old_serial_numbers: Vec<<Self as Transaction>::SerialNumber>,
        new_commitments: Vec<<Self as Transaction>::Commitment>,
        memorandum: <Self as Transaction>::Memorandum,
        ledger_digest: MerkleTreeDigest<C::MerkleParameters>,
        transaction_proof: <C::OuterSNARK as SNARK>::Proof,
        predicate_commitment: <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
        local_data_commitment: <C::LocalDataCommitment as CommitmentScheme>::Output,
        value_balance: i64,
        network_id: u8,
        signatures: Vec<<C::AccountSignature as SignatureScheme>::Output>,
    ) -> Self {
        Self {
            old_serial_numbers,
            new_commitments,
            memorandum,
            ledger_digest,
            transaction_proof,
            predicate_commitment,
            local_data_commitment,
            value_balance,
            network_id,
            signatures,
        }
    }
}

impl<C: BaseDPCComponents> Transaction for DPCTransaction<C> {
    type Commitment = <C::RecordCommitment as CommitmentScheme>::Output;
    type Digest = MerkleTreeDigest<C::MerkleParameters>;
    type LocalDataCommitment = <C::LocalDataCommitment as CommitmentScheme>::Output;
    type Memorandum = [u8; 32];
    type PredicateCommitment = <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output;
    type SerialNumber = <C::AccountSignature as SignatureScheme>::PublicKey;

    /// Transaction id = Hash of (serial numbers || commitments || memo)
    fn transaction_id(&self) -> Result<[u8; 32], TransactionError> {
        let mut pre_image_bytes: Vec<u8> = vec![];

        for sn in self.old_serial_numbers() {
            pre_image_bytes.extend(&to_bytes![sn]?);
        }

        for cm in self.new_commitments() {
            pre_image_bytes.extend(&to_bytes![cm]?);
        }

        pre_image_bytes.extend(self.memorandum());

        let mut h = b2s::new();
        h.input(&pre_image_bytes);

        let mut result = [0u8; 32];
        result.copy_from_slice(&h.result());
        Ok(result)
    }

    fn network_id(&self) -> u8 {
        self.network_id
    }

    fn ledger_digest(&self) -> &Self::Digest {
        &self.ledger_digest
    }

    fn old_serial_numbers(&self) -> &[Self::SerialNumber] {
        self.old_serial_numbers.as_slice()
    }

    fn new_commitments(&self) -> &[Self::Commitment] {
        self.new_commitments.as_slice()
    }

    fn predicate_commitment(&self) -> &Self::PredicateCommitment {
        &self.predicate_commitment
    }

    fn local_data_commitment(&self) -> &Self::LocalDataCommitment {
        &self.local_data_commitment
    }

    fn value_balance(&self) -> i64 {
        self.value_balance
    }

    fn memorandum(&self) -> &Self::Memorandum {
        &self.memorandum
    }

    fn size(&self) -> usize {
        let transaction_bytes = to_bytes![self].unwrap();
        transaction_bytes.len()
    }
}

impl<C: BaseDPCComponents> ToBytes for DPCTransaction<C> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        variable_length_integer(self.old_serial_numbers.len() as u64).write(&mut writer)?;
        for old_serial_number in &self.old_serial_numbers {
            CanonicalSerialize::serialize(old_serial_number, &mut writer).unwrap();
        }

        variable_length_integer(self.new_commitments.len() as u64).write(&mut writer)?;
        for new_commitment in &self.new_commitments {
            new_commitment.write(&mut writer)?;
        }

        self.memorandum.write(&mut writer)?;

        self.ledger_digest.write(&mut writer)?;
        self.transaction_proof.write(&mut writer)?;
        self.predicate_commitment.write(&mut writer)?;
        self.local_data_commitment.write(&mut writer)?;
        self.value_balance.write(&mut writer)?;
        self.network_id.write(&mut writer)?;

        variable_length_integer(self.signatures.len() as u64).write(&mut writer)?;
        for signature in &self.signatures {
            signature.write(&mut writer)?;
        }

        Ok(())
    }
}

impl<C: BaseDPCComponents> FromBytes for DPCTransaction<C> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let num_old_serial_numbers = read_variable_length_integer(&mut reader)?;
        let mut old_serial_numbers = vec![];
        for _ in 0..num_old_serial_numbers {
            let old_serial_number: <C::AccountSignature as SignatureScheme>::PublicKey =
                CanonicalDeserialize::deserialize(&mut reader).unwrap();

            old_serial_numbers.push(old_serial_number);
        }

        let num_new_commitments = read_variable_length_integer(&mut reader)?;
        let mut new_commitments = vec![];
        for _ in 0..num_new_commitments {
            let new_commitment: <C::RecordCommitment as CommitmentScheme>::Output = FromBytes::read(&mut reader)?;
            new_commitments.push(new_commitment);
        }

        let memorandum: [u8; 32] = FromBytes::read(&mut reader)?;

        let ledger_digest: MerkleTreeDigest<C::MerkleParameters> = FromBytes::read(&mut reader)?;
        let transaction_proof: <C::OuterSNARK as SNARK>::Proof = FromBytes::read(&mut reader)?;
        let predicate_commitment: <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output =
            FromBytes::read(&mut reader)?;
        let local_data_commitment: <C::LocalDataCommitment as CommitmentScheme>::Output = FromBytes::read(&mut reader)?;

        let value_balance: i64 = FromBytes::read(&mut reader)?;
        let network_id: u8 = FromBytes::read(&mut reader)?;

        let num_signatures = read_variable_length_integer(&mut reader)?;
        let mut signatures = vec![];
        for _ in 0..num_signatures {
            let signature: <C::AccountSignature as SignatureScheme>::Output = FromBytes::read(&mut reader)?;
            signatures.push(signature);
        }

        Ok(Self {
            network_id,
            ledger_digest,
            old_serial_numbers,
            new_commitments,
            predicate_commitment,
            local_data_commitment,
            value_balance,
            signatures,
            transaction_proof,
            memorandum,
        })
    }
}

impl<C: BaseDPCComponents> fmt::Debug for DPCTransaction<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "DPCTransaction {{ network_id: {:?}, digest: {:?}, old_serial_numbers: {:?}, new_commitments: {:?}, predicate_commitment: {:?}, local_data_commitment: {:?}, value_balance: {:?}, signatures: {:?}, transaction_proof: {:?}, memorandum: {:?} }}",
            self.network_id,
            self.ledger_digest,
            self.old_serial_numbers,
            self.new_commitments,
            self.predicate_commitment,
            self.local_data_commitment,
            self.value_balance,
            self.signatures,
            self.transaction_proof,
            self.memorandum,
        )
    }
}
