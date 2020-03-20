use crate::{
    dpc::base_dpc::{binding_signature::BindingSignature, BaseDPCComponents},
    Transaction,
};
use snarkos_algorithms::merkle_tree::MerkleTreeDigest;
use snarkos_models::algorithms::{CommitmentScheme, SignatureScheme, SNARK};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    variable_length_integer::{read_variable_length_integer, variable_length_integer},
};

use std::io::{Read, Result as IoResult, Write};

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

    pub input_value_commitments: Vec<[u8; 32]>,
    pub output_value_commitments: Vec<[u8; 32]>,
    pub value_balance: u64,
    pub binding_signature: BindingSignature,

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
        input_value_commitments: Vec<[u8; 32]>,
        output_value_commitments: Vec<[u8; 32]>,
        value_balance: u64,
        binding_signature: BindingSignature,
        signatures: Vec<<C::Signature as SignatureScheme>::Output>,
    ) -> Self {
        let stuff = DPCStuff {
            digest,
            inner_proof,
            predicate_proof,
            predicate_commitment,
            local_data_commitment,
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

impl<C: BaseDPCComponents> ToBytes for DPCTransaction<C> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        variable_length_integer(self.old_serial_numbers.len() as u64).write(&mut writer)?;
        for old_serial_number in &self.old_serial_numbers {
            old_serial_number.write(&mut writer)?;
        }

        variable_length_integer(self.new_commitments.len() as u64).write(&mut writer)?;
        for new_commitment in &self.new_commitments {
            new_commitment.write(&mut writer)?;
        }

        self.memorandum.write(&mut writer)?;

        self.stuff.write(&mut writer)
    }
}

impl<C: BaseDPCComponents> FromBytes for DPCTransaction<C> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let num_old_serial_numbers = read_variable_length_integer(&mut reader)?;
        let mut old_serial_numbers = vec![];
        for _ in 0..num_old_serial_numbers {
            let old_serial_number: <C::Signature as SignatureScheme>::PublicKey = FromBytes::read(&mut reader)?;
            old_serial_numbers.push(old_serial_number);
        }

        let num_new_commitments = read_variable_length_integer(&mut reader)?;
        let mut new_commitments = vec![];
        for _ in 0..num_new_commitments {
            let new_commitment: <C::RecordCommitment as CommitmentScheme>::Output = FromBytes::read(&mut reader)?;
            new_commitments.push(new_commitment);
        }

        let memorandum: [u8; 32] = FromBytes::read(&mut reader)?;
        let stuff: DPCStuff<C> = FromBytes::read(&mut reader)?;

        Ok(Self {
            old_serial_numbers,
            new_commitments,
            memorandum,
            stuff,
        })
    }
}

impl<C: BaseDPCComponents> ToBytes for DPCStuff<C> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.digest.write(&mut writer)?;
        self.inner_proof.write(&mut writer)?;
        self.predicate_proof.write(&mut writer)?;
        self.predicate_commitment.write(&mut writer)?;
        self.local_data_commitment.write(&mut writer)?;

        variable_length_integer(self.input_value_commitments.len() as u64).write(&mut writer)?;
        for input_value_commitment in &self.input_value_commitments {
            input_value_commitment.write(&mut writer)?;
        }

        variable_length_integer(self.output_value_commitments.len() as u64).write(&mut writer)?;
        for output_value_commitment in &self.output_value_commitments {
            output_value_commitment.write(&mut writer)?;
        }

        self.value_balance.write(&mut writer)?;
        self.binding_signature.write(&mut writer)?;

        variable_length_integer(self.signatures.len() as u64).write(&mut writer)?;
        for signature in &self.signatures {
            signature.write(&mut writer)?;
        }

        Ok(())
    }
}

impl<C: BaseDPCComponents> FromBytes for DPCStuff<C> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let digest: MerkleTreeDigest<C::MerkleParameters> = FromBytes::read(&mut reader)?;
        let inner_proof: <C::InnerSNARK as SNARK>::Proof = FromBytes::read(&mut reader)?;
        let predicate_proof: <C::OuterSNARK as SNARK>::Proof = FromBytes::read(&mut reader)?;
        let predicate_commitment: <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output =
            FromBytes::read(&mut reader)?;
        let local_data_commitment: <C::LocalDataCommitment as CommitmentScheme>::Output = FromBytes::read(&mut reader)?;

        let num_input_value_commitments = read_variable_length_integer(&mut reader)?;
        let mut input_value_commitments = vec![];
        for _ in 0..num_input_value_commitments {
            let input_value_commitment: [u8; 32] = FromBytes::read(&mut reader)?;
            input_value_commitments.push(input_value_commitment);
        }

        let num_output_value_commitments = read_variable_length_integer(&mut reader)?;
        let mut output_value_commitments = vec![];
        for _ in 0..num_output_value_commitments {
            let output_value_commitment: [u8; 32] = FromBytes::read(&mut reader)?;
            output_value_commitments.push(output_value_commitment);
        }

        let value_balance: u64 = FromBytes::read(&mut reader)?;

        let binding_signature: BindingSignature = FromBytes::read(&mut reader)?;

        let num_signatures = read_variable_length_integer(&mut reader)?;
        let mut signatures = vec![];
        for _ in 0..num_signatures {
            let signature: <C::Signature as SignatureScheme>::Output = FromBytes::read(&mut reader)?;
            signatures.push(signature);
        }

        Ok(Self {
            digest,
            inner_proof,
            predicate_proof,
            predicate_commitment,
            local_data_commitment,
            input_value_commitments,
            output_value_commitments,
            value_balance,
            binding_signature,
            signatures,
        })
    }
}
