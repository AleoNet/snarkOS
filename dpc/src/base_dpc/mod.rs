use crate::base_dpc::record_payload::RecordPayload;
use snarkos_algorithms::{
    commitment_tree::CommitmentMerkleTree,
    merkle_tree::{MerklePath, MerkleTreeDigest},
};
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    algorithms::{
        CommitmentScheme,
        EncryptionScheme,
        LoadableMerkleParameters,
        MerkleParameters,
        SignatureScheme,
        CRH,
        PRF,
        SNARK,
    },
    curves::{Group, MontgomeryModelParameters, ProjectiveCurve, TEModelParameters},
    dpc::{DPCComponents, DPCScheme, Record},
    gadgets::algorithms::{CRHGadget, SNARKVerifierGadget},
    objects::{AccountScheme, LedgerScheme, Transaction},
};
use snarkos_objects::{Account, AccountAddress, AccountPrivateKey, AleoAmount, Network};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    has_duplicates,
    rand::UniformRand,
    to_bytes,
};

use itertools::Itertools;
use rand::Rng;
use std::marker::PhantomData;

pub mod inner_circuit;
pub use inner_circuit::*;

pub mod outer_circuit;
pub use outer_circuit::*;

pub mod parameters;
pub use parameters::*;

pub mod program;
pub use program::*;

pub mod record;
pub use record::*;

pub mod transaction;
pub use transaction::*;

pub mod instantiated;

#[cfg(test)]
mod test;

///////////////////////////////////////////////////////////////////////////////

/// Trait that stores all information about the components of a Plain DPC
/// scheme. Simplifies the interface of Plain DPC by wrapping all these into
/// one.
pub trait BaseDPCComponents: DPCComponents {
    /// Ledger digest type.
    type MerkleParameters: LoadableMerkleParameters;
    type MerkleHashGadget: CRHGadget<<Self::MerkleParameters as MerkleParameters>::H, Self::InnerField>;

    /// Group and Model Parameters for record encryption
    type EncryptionGroup: Group + ProjectiveCurve;
    type EncryptionModelParameters: MontgomeryModelParameters + TEModelParameters;

    /// SNARK for non-proof-verification checks
    type InnerSNARK: SNARK<
        Circuit = InnerCircuit<Self>,
        AssignedCircuit = InnerCircuit<Self>,
        VerifierInput = InnerCircuitVerifierInput<Self>,
    >;

    /// SNARK Verifier gadget for the inner snark
    type InnerSNARKGadget: SNARKVerifierGadget<Self::InnerSNARK, Self::OuterField>;

    /// SNARK for proof-verification checks
    type OuterSNARK: SNARK<
        Circuit = OuterCircuit<Self>,
        AssignedCircuit = OuterCircuit<Self>,
        VerifierInput = OuterCircuitVerifierInput<Self>,
    >;

    // TODO (raychu86) remove these from BaseDPCComponents
    /// SNARK for the Noop "always-accept" that does nothing with its input.
    type NoopProgramSNARK: SNARK<
        Circuit = NoopCircuit<Self>,
        AssignedCircuit = NoopCircuit<Self>,
        VerifierInput = ProgramLocalData<Self>,
    >;

    /// SNARK Verifier gadget for the "dummy program" that does nothing with its input.
    type ProgramSNARKGadget: SNARKVerifierGadget<Self::NoopProgramSNARK, Self::OuterField>;
}

///////////////////////////////////////////////////////////////////////////////

pub struct DPC<Components: BaseDPCComponents> {
    _components: PhantomData<Components>,
}

/// Returned by `BaseDPC::execute_offline`. Stores data required to produce the
/// final transaction after `execute_offline` has created old serial numbers,
/// new records and commitments. For convenience, it also
/// stores references to existing information like old records and secret keys.
pub struct ExecuteContext<Components: BaseDPCComponents> {
    system_parameters: SystemParameters<Components>,

    // Old record stuff
    old_account_private_keys: Vec<AccountPrivateKey<Components>>,
    old_records: Vec<DPCRecord<Components>>,
    old_serial_numbers: Vec<<Components::AccountSignature as SignatureScheme>::PublicKey>,
    old_randomizers: Vec<Vec<u8>>,

    // New record stuff
    new_records: Vec<DPCRecord<Components>>,
    new_sn_nonce_randomness: Vec<[u8; 32]>,
    new_commitments: Vec<<Components::RecordCommitment as CommitmentScheme>::Output>,

    new_records_encryption_randomness: Vec<<Components::AccountEncryption as EncryptionScheme>::Randomness>,
    new_encrypted_records: Vec<EncryptedRecord<Components>>,
    new_encrypted_record_hashes: Vec<<Components::EncryptedRecordCRH as CRH>::Output>,

    // Program and local data root and randomness
    program_commitment: <Components::ProgramVerificationKeyCommitment as CommitmentScheme>::Output,
    program_randomness: <Components::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness,

    local_data_merkle_tree: CommitmentMerkleTree<Components::LocalDataCommitment, Components::LocalDataCRH>,
    local_data_commitment_randomizers: Vec<<Components::LocalDataCommitment as CommitmentScheme>::Randomness>,

    value_balance: AleoAmount,
    memorandum: <DPCTransaction<Components> as Transaction>::Memorandum,
    network_id: u8,
}

impl<Components: BaseDPCComponents> ExecuteContext<Components> {
    pub fn into_local_data(&self) -> LocalData<Components> {
        LocalData {
            system_parameters: self.system_parameters.clone(),

            old_records: self.old_records.to_vec(),
            old_serial_numbers: self.old_serial_numbers.to_vec(),

            new_records: self.new_records.to_vec(),

            local_data_merkle_tree: self.local_data_merkle_tree.clone(),
            local_data_commitment_randomizers: self.local_data_commitment_randomizers.clone(),

            memorandum: self.memorandum.clone(),
            network_id: self.network_id,
        }
    }
}

/// Stores local data required to produce program proofs.
pub struct LocalData<Components: BaseDPCComponents> {
    pub system_parameters: SystemParameters<Components>,

    // Old records and serial numbers
    pub old_records: Vec<DPCRecord<Components>>,
    pub old_serial_numbers: Vec<<Components::AccountSignature as SignatureScheme>::PublicKey>,

    // New records
    pub new_records: Vec<DPCRecord<Components>>,

    // Commitment to the above information.
    pub local_data_merkle_tree: CommitmentMerkleTree<Components::LocalDataCommitment, Components::LocalDataCRH>,
    pub local_data_commitment_randomizers: Vec<<Components::LocalDataCommitment as CommitmentScheme>::Randomness>,

    pub memorandum: <DPCTransaction<Components> as Transaction>::Memorandum,
    pub network_id: u8,
}

///////////////////////////////////////////////////////////////////////////////

impl<Components: BaseDPCComponents> DPC<Components> {
    pub fn generate_system_parameters<R: Rng>(rng: &mut R) -> Result<SystemParameters<Components>, DPCError> {
        let time = start_timer!(|| "Account commitment scheme setup");
        let account_commitment = Components::AccountCommitment::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Account encryption scheme setup");
        let account_encryption = Components::AccountEncryption::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Account signature setup");
        let account_signature = Components::AccountSignature::setup(rng)?;
        end_timer!(time);

        let time = start_timer!(|| "Encrypted record CRH setup");
        let encrypted_record_crh = Components::EncryptedRecordCRH::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Inner SNARK verification key CRH setup");
        let inner_snark_verification_key_crh = Components::InnerSNARKVerificationKeyCRH::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Local data commitment setup");
        let local_data_commitment = Components::LocalDataCommitment::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Local data CRH setup");
        let local_data_crh = Components::LocalDataCRH::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Program verification key CRH setup");
        let program_verification_key_crh = Components::ProgramVerificationKeyCRH::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Program verification key commitment setup");
        let program_verification_key_commitment = Components::ProgramVerificationKeyCommitment::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Record commitment scheme setup");
        let record_commitment = Components::RecordCommitment::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Serial nonce CRH setup");
        let serial_number_nonce = Components::SerialNumberNonceCRH::setup(rng);
        end_timer!(time);

        Ok(SystemParameters {
            account_commitment,
            account_encryption,
            account_signature,
            encrypted_record_crh,
            inner_snark_verification_key_crh,
            local_data_crh,
            local_data_commitment,
            program_verification_key_commitment,
            program_verification_key_crh,
            record_commitment,
            serial_number_nonce,
        })
    }

    pub fn generate_noop_program_snark_parameters<R: Rng>(
        system_parameters: &SystemParameters<Components>,
        rng: &mut R,
    ) -> Result<NoopProgramSNARKParameters<Components>, DPCError> {
        let (pk, pvk) = Components::NoopProgramSNARK::setup(NoopCircuit::blank(system_parameters), rng)?;

        Ok(NoopProgramSNARKParameters {
            proving_key: pk,
            verification_key: pvk.into(),
        })
    }

    pub fn generate_sn(
        system_parameters: &SystemParameters<Components>,
        record: &DPCRecord<Components>,
        account_private_key: &AccountPrivateKey<Components>,
    ) -> Result<(<Components::AccountSignature as SignatureScheme>::PublicKey, Vec<u8>), DPCError> {
        let sn_time = start_timer!(|| "Generate serial number");
        let sk_prf = &account_private_key.sk_prf;
        let sn_nonce = to_bytes!(record.serial_number_nonce())?;
        // Compute the serial number.
        let prf_input = FromBytes::read(sn_nonce.as_slice())?;
        let prf_seed = FromBytes::read(to_bytes!(sk_prf)?.as_slice())?;
        let sig_and_pk_randomizer = to_bytes![Components::PRF::evaluate(&prf_seed, &prf_input)?]?;

        let sn = Components::AccountSignature::randomize_public_key(
            &system_parameters.account_signature,
            &account_private_key.pk_sig(&system_parameters.account_signature)?,
            &sig_and_pk_randomizer,
        )?;
        end_timer!(sn_time);
        Ok((sn, sig_and_pk_randomizer))
    }

    pub fn generate_record<R: Rng>(
        system_parameters: &SystemParameters<Components>,
        sn_nonce: &<Components::SerialNumberNonceCRH as CRH>::Output,
        owner: &AccountAddress<Components>,
        is_dummy: bool,
        value: u64,
        payload: &RecordPayload,
        birth_program_id: &Vec<u8>,
        death_program_id: &Vec<u8>,
        rng: &mut R,
    ) -> Result<DPCRecord<Components>, DPCError> {
        let record_time = start_timer!(|| "Generate record");
        // Sample new commitment randomness.
        let commitment_randomness = <Components::RecordCommitment as CommitmentScheme>::Randomness::rand(rng);

        // Total = 32 + 1 + 8 + 32 + 48 + 48 + 32 = 201 bytes
        let commitment_input = to_bytes![
            owner,            // 256 bits = 32 bytes
            is_dummy,         // 1 bit = 1 byte
            value,            // 64 bits = 8 bytes
            payload,          // 256 bits = 32 bytes
            birth_program_id, // 384 bits = 48 bytes
            death_program_id, // 384 bits = 48 bytes
            sn_nonce          // 256 bits = 32 bytes
        ]?;

        let commitment = Components::RecordCommitment::commit(
            &system_parameters.record_commitment,
            &commitment_input,
            &commitment_randomness,
        )?;

        let record = DPCRecord {
            owner: owner.clone(),
            is_dummy,
            value,
            payload: payload.clone(),
            birth_program_id: birth_program_id.to_vec(),
            death_program_id: death_program_id.to_vec(),
            serial_number_nonce: sn_nonce.clone(),
            commitment,
            commitment_randomness,
            _components: PhantomData,
        };
        end_timer!(record_time);
        Ok(record)
    }
}

impl<Components: BaseDPCComponents, L: LedgerScheme> DPCScheme<L> for DPC<Components>
where
    L: LedgerScheme<
        Commitment = <Components::RecordCommitment as CommitmentScheme>::Output,
        MerkleParameters = Components::MerkleParameters,
        MerklePath = MerklePath<Components::MerkleParameters>,
        MerkleTreeDigest = MerkleTreeDigest<Components::MerkleParameters>,
        SerialNumber = <Components::AccountSignature as SignatureScheme>::PublicKey,
        Transaction = DPCTransaction<Components>,
    >,
{
    type Account = Account<Components>;
    type ExecuteContext = ExecuteContext<Components>;
    type LocalData = LocalData<Components>;
    type Metadata = [u8; 32];
    type Parameters = PublicParameters<Components>;
    type Payload = <Self::Record as Record>::Payload;
    type PrivateProgramInput = PrivateProgramInput;
    type Record = DPCRecord<Components>;
    type SystemParameters = SystemParameters<Components>;
    type Transaction = DPCTransaction<Components>;

    fn setup<R: Rng>(
        ledger_parameters: &Components::MerkleParameters,
        rng: &mut R,
    ) -> Result<Self::Parameters, DPCError> {
        let setup_time = start_timer!(|| "BaseDPC::setup");
        let system_parameters = Self::generate_system_parameters(rng)?;

        let program_snark_setup_time = start_timer!(|| "Dummy program SNARK setup");
        let noop_program_snark_parameters = Self::generate_noop_program_snark_parameters(&system_parameters, rng)?;
        let program_snark_proof = Components::NoopProgramSNARK::prove(
            &noop_program_snark_parameters.proving_key,
            NoopCircuit::blank(&system_parameters),
            rng,
        )?;
        end_timer!(program_snark_setup_time);

        let program_snark_vk_and_proof = PrivateProgramInput {
            verification_key: to_bytes![noop_program_snark_parameters.verification_key]?,
            proof: to_bytes![program_snark_proof]?,
        };

        let snark_setup_time = start_timer!(|| "Execute inner SNARK setup");
        let inner_snark_parameters =
            Components::InnerSNARK::setup(InnerCircuit::blank(&system_parameters, ledger_parameters), rng)?;
        end_timer!(snark_setup_time);

        let snark_setup_time = start_timer!(|| "Execute outer SNARK setup");
        let inner_snark_vk: <Components::InnerSNARK as SNARK>::VerificationParameters =
            inner_snark_parameters.1.clone().into();
        let inner_snark_proof = Components::InnerSNARK::prove(
            &inner_snark_parameters.0,
            InnerCircuit::blank(&system_parameters, ledger_parameters),
            rng,
        )?;

        let outer_snark_parameters = Components::OuterSNARK::setup(
            OuterCircuit::blank(
                &system_parameters,
                ledger_parameters,
                &inner_snark_vk,
                &inner_snark_proof,
                &program_snark_vk_and_proof,
            ),
            rng,
        )?;
        end_timer!(snark_setup_time);
        end_timer!(setup_time);

        let inner_snark_parameters = (Some(inner_snark_parameters.0), inner_snark_parameters.1);
        let outer_snark_parameters = (Some(outer_snark_parameters.0), outer_snark_parameters.1);

        Ok(PublicParameters {
            system_parameters,
            noop_program_snark_parameters,
            inner_snark_parameters,
            outer_snark_parameters,
        })
    }

    fn create_account<R: Rng>(parameters: &Self::Parameters, rng: &mut R) -> Result<Self::Account, DPCError> {
        let time = start_timer!(|| "BaseDPC::create_account");

        let account_signature_parameters = &parameters.system_parameters.account_signature;
        let commitment_parameters = &parameters.system_parameters.account_commitment;
        let encryption_parameters = &parameters.system_parameters.account_encryption;
        let account = Account::new(
            account_signature_parameters,
            commitment_parameters,
            encryption_parameters,
            rng,
        )?;

        end_timer!(time);

        Ok(account)
    }

    fn execute_offline<R: Rng>(
        parameters: &Self::SystemParameters,
        old_records: &[Self::Record],
        old_account_private_keys: &[<Self::Account as AccountScheme>::AccountPrivateKey],
        new_record_owners: &[<Self::Account as AccountScheme>::AccountAddress],
        new_is_dummy_flags: &[bool],
        new_values: &[u64],
        new_payloads: &[Self::Payload],
        new_birth_program_ids: &[Vec<u8>],
        new_death_program_ids: &[Vec<u8>],
        memorandum: &<Self::Transaction as Transaction>::Memorandum,
        network_id: u8,
        rng: &mut R,
    ) -> Result<Self::ExecuteContext, DPCError> {
        assert_eq!(Components::NUM_INPUT_RECORDS, old_records.len());
        assert_eq!(Components::NUM_INPUT_RECORDS, old_account_private_keys.len());

        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_record_owners.len());
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_is_dummy_flags.len());
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_payloads.len());
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_birth_program_ids.len());
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_death_program_ids.len());

        let mut old_serial_numbers = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        let mut old_randomizers = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        let mut joint_serial_numbers = Vec::new();
        let mut old_death_program_ids = Vec::new();

        let mut value_balance = AleoAmount::ZERO;

        // Compute the ledger membership witness and serial number from the old records.
        for (i, record) in old_records.iter().enumerate() {
            let input_record_time = start_timer!(|| format!("Process input record {}", i));

            if !record.is_dummy() {
                value_balance = value_balance.add(AleoAmount::from_bytes(record.value() as i64));
            }

            let (sn, randomizer) = Self::generate_sn(&parameters, record, &old_account_private_keys[i])?;
            joint_serial_numbers.extend_from_slice(&to_bytes![sn]?);
            old_serial_numbers.push(sn);
            old_randomizers.push(randomizer);
            old_death_program_ids.push(record.death_program_id().to_vec());

            end_timer!(input_record_time);
        }

        let mut new_records = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);
        let mut new_commitments = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);
        let mut new_sn_nonce_randomness = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);

        // Generate new records and commitments for them.
        for j in 0..Components::NUM_OUTPUT_RECORDS {
            let output_record_time = start_timer!(|| format!("Process output record {}", j));
            let sn_nonce_time = start_timer!(|| "Generate serial number nonce");

            // Sample randomness sn_randomness for the CRH input.
            let sn_randomness: [u8; 32] = rng.gen();

            let crh_input = to_bytes![j as u8, sn_randomness, joint_serial_numbers]?;
            let sn_nonce = Components::SerialNumberNonceCRH::hash(&parameters.serial_number_nonce, &crh_input)?;

            end_timer!(sn_nonce_time);

            let record = Self::generate_record(
                parameters,
                &sn_nonce,
                &new_record_owners[j],
                new_is_dummy_flags[j],
                new_values[j],
                &new_payloads[j],
                &new_birth_program_ids[j],
                &new_death_program_ids[j],
                rng,
            )?;

            if !record.is_dummy() {
                value_balance = value_balance.sub(AleoAmount::from_bytes(record.value() as i64));
            }

            new_commitments.push(record.commitment().clone());
            new_sn_nonce_randomness.push(sn_randomness);
            new_records.push(record);

            end_timer!(output_record_time);
        }

        // TODO (raychu86) Add index and program register inputs + outputs to local data commitment leaves
        let local_data_merkle_tree_timer = start_timer!(|| "Compute local data merkle tree");

        let mut local_data_commitment_randomizers = vec![];

        let mut old_record_commitments = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        for i in 0..Components::NUM_INPUT_RECORDS {
            let record = &old_records[i];
            let input_bytes = to_bytes![old_serial_numbers[i], record.commitment(), memorandum, network_id]?;

            let commitment_randomness = <Components::LocalDataCommitment as CommitmentScheme>::Randomness::rand(rng);
            let commitment = Components::LocalDataCommitment::commit(
                &parameters.local_data_commitment,
                &input_bytes,
                &commitment_randomness,
            )?;

            old_record_commitments.push(commitment);
            local_data_commitment_randomizers.push(commitment_randomness);
        }

        let mut new_record_commitments = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);
        for j in 0..Components::NUM_OUTPUT_RECORDS {
            let record = &new_records[j];
            let input_bytes = to_bytes![record.commitment(), memorandum, network_id]?;

            let commitment_randomness = <Components::LocalDataCommitment as CommitmentScheme>::Randomness::rand(rng);
            let commitment = Components::LocalDataCommitment::commit(
                &parameters.local_data_commitment,
                &input_bytes,
                &commitment_randomness,
            )?;

            new_record_commitments.push(commitment);
            local_data_commitment_randomizers.push(commitment_randomness);
        }

        let leaves = [
            old_record_commitments[0].clone(),
            old_record_commitments[1].clone(),
            new_record_commitments[0].clone(),
            new_record_commitments[1].clone(),
        ];
        let local_data_merkle_tree = CommitmentMerkleTree::new(parameters.local_data_crh.clone(), &leaves)?;

        end_timer!(local_data_merkle_tree_timer);

        let program_comm_timer = start_timer!(|| "Compute program commitment");
        let (program_commitment, program_randomness) = {
            let mut input = Vec::new();
            for id in old_death_program_ids {
                input.extend_from_slice(&id);
            }

            for id in new_birth_program_ids {
                input.extend_from_slice(&id);
            }
            let program_randomness =
                <Components::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness::rand(rng);
            let program_commitment = Components::ProgramVerificationKeyCommitment::commit(
                &parameters.program_verification_key_commitment,
                &input,
                &program_randomness,
            )?;
            (program_commitment, program_randomness)
        };
        end_timer!(program_comm_timer);

        // Encrypt the new records

        let mut new_records_encryption_randomness = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);
        let mut new_encrypted_records = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);

        for record in &new_records {
            let (record_encryption_randomness, encrypted_record) =
                RecordEncryption::encrypt_record(&parameters, record, rng)?;

            new_records_encryption_randomness.push(record_encryption_randomness);
            new_encrypted_records.push(encrypted_record);
        }

        // Construct the ciphertext hashes

        let mut new_encrypted_record_hashes = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);
        for encrypted_record in &new_encrypted_records {
            let encrypted_record_hash = RecordEncryption::encrypted_record_hash(&parameters, &encrypted_record)?;

            new_encrypted_record_hashes.push(encrypted_record_hash);
        }

        let context = ExecuteContext {
            system_parameters: parameters.clone(),

            old_records: old_records.to_vec(),
            old_account_private_keys: old_account_private_keys.to_vec(),
            old_serial_numbers,
            old_randomizers,

            new_records,
            new_sn_nonce_randomness,
            new_commitments,

            new_records_encryption_randomness,
            new_encrypted_records,
            new_encrypted_record_hashes,

            program_commitment,
            program_randomness,
            local_data_merkle_tree,
            local_data_commitment_randomizers,

            value_balance,
            memorandum: memorandum.clone(),
            network_id,
        };
        Ok(context)
    }

    fn execute_online<R: Rng>(
        parameters: &Self::Parameters,
        context: Self::ExecuteContext,
        old_death_program_proofs: &[Self::PrivateProgramInput],
        new_birth_program_proofs: &[Self::PrivateProgramInput],
        ledger: &L,
        rng: &mut R,
    ) -> Result<(Vec<Self::Record>, Self::Transaction), DPCError> {
        assert_eq!(Components::NUM_INPUT_RECORDS, old_death_program_proofs.len());
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_birth_program_proofs.len());

        let exec_time = start_timer!(|| "BaseDPC::execute_online");

        let ExecuteContext {
            system_parameters,

            old_records,
            old_account_private_keys,
            old_serial_numbers,
            old_randomizers,

            new_records,
            new_sn_nonce_randomness,
            new_commitments,

            new_records_encryption_randomness,
            new_encrypted_records,
            new_encrypted_record_hashes,

            program_commitment,
            program_randomness,
            local_data_merkle_tree,
            local_data_commitment_randomizers,
            value_balance,
            memorandum,
            network_id,
        } = context;

        let local_data_root = local_data_merkle_tree.root();

        let old_death_program_attributes = old_death_program_proofs;
        let new_birth_program_attributes = new_birth_program_proofs;

        // Construct the ledger witnesses

        let ledger_digest = ledger.digest().expect("could not get digest");

        // Generate the ledger membership witnesses
        let mut old_witnesses = Vec::with_capacity(Components::NUM_INPUT_RECORDS);

        // Compute the ledger membership witness and serial number from the old records.
        for record in old_records.iter() {
            if record.is_dummy() {
                old_witnesses.push(MerklePath::default());
            } else {
                let witness = ledger.prove_cm(&record.commitment())?;
                old_witnesses.push(witness);
            }
        }

        // Generate Schnorr signature on transaction data
        // TODO (raychu86) Remove ledger_digest from signature and move the schnorr signing into `execute_offline`
        let signature_time = start_timer!(|| "Sign and randomize transaction contents");

        let signature_message = to_bytes![
            network_id,
            ledger_digest,
            old_serial_numbers,
            new_commitments,
            program_commitment,
            local_data_root,
            value_balance,
            memorandum
        ]?;

        let mut signatures = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        for i in 0..Components::NUM_INPUT_RECORDS {
            let sk_sig = &old_account_private_keys[i].sk_sig;
            let randomizer = &old_randomizers[i];

            // Sign the transaction data
            let account_signature = Components::AccountSignature::sign(
                &system_parameters.account_signature,
                sk_sig,
                &signature_message,
                rng,
            )?;

            // Randomize the signature
            let randomized_signature = Components::AccountSignature::randomize_signature(
                &system_parameters.account_signature,
                &account_signature,
                randomizer,
            )?;

            signatures.push(randomized_signature);
        }

        end_timer!(signature_time);

        // Prepare record encryption components used in the inner SNARK

        let mut new_records_encryption_gadget_components = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);

        for (record, ciphertext_randomness) in new_records.iter().zip_eq(&new_records_encryption_randomness) {
            let record_encryption_gadget_components = RecordEncryption::prepare_encryption_gadget_components(
                &system_parameters,
                &record,
                ciphertext_randomness,
            )?;

            new_records_encryption_gadget_components.push(record_encryption_gadget_components);
        }

        let inner_proof = {
            let circuit = InnerCircuit::new(
                &parameters.system_parameters,
                ledger.parameters(),
                &ledger_digest,
                &old_records,
                &old_witnesses,
                &old_account_private_keys,
                &old_serial_numbers,
                &new_records,
                &new_sn_nonce_randomness,
                &new_commitments,
                &new_records_encryption_randomness,
                &new_records_encryption_gadget_components,
                &new_encrypted_record_hashes,
                &program_commitment,
                &program_randomness,
                &local_data_root,
                &local_data_commitment_randomizers,
                &memorandum,
                value_balance,
                network_id,
            );

            let inner_snark_parameters = match &parameters.inner_snark_parameters.0 {
                Some(inner_snark_parameters) => inner_snark_parameters,
                None => return Err(DPCError::MissingInnerSnarkProvingParameters),
            };

            Components::InnerSNARK::prove(&inner_snark_parameters, circuit, rng)?
        };

        // Verify that the inner proof passes
        {
            let input = InnerCircuitVerifierInput {
                system_parameters: parameters.system_parameters.clone(),
                ledger_parameters: ledger.parameters().clone(),
                ledger_digest: ledger_digest.clone(),
                old_serial_numbers: old_serial_numbers.clone(),
                new_commitments: new_commitments.clone(),
                new_encrypted_record_hashes: new_encrypted_record_hashes.clone(),
                memo: memorandum.clone(),
                program_commitment: program_commitment.clone(),
                local_data_root: local_data_root.clone(),
                value_balance,
                network_id,
            };

            let verification_key = &parameters.inner_snark_parameters.1;

            assert!(Components::InnerSNARK::verify(verification_key, &input, &inner_proof)?);
        }

        let inner_snark_vk: <Components::InnerSNARK as SNARK>::VerificationParameters =
            parameters.inner_snark_parameters.1.clone().into();

        let inner_snark_id = <Components::InnerSNARKVerificationKeyCRH as CRH>::hash(
            &parameters.system_parameters.inner_snark_verification_key_crh,
            &to_bytes![inner_snark_vk]?,
        )?;

        let transaction_proof = {
            let ledger_parameters = ledger.parameters();

            let circuit = OuterCircuit::new(
                &parameters.system_parameters,
                ledger_parameters,
                &ledger_digest,
                &old_serial_numbers,
                &new_commitments,
                &new_encrypted_record_hashes,
                &memorandum,
                value_balance,
                network_id,
                &inner_snark_vk,
                &inner_proof,
                &old_death_program_attributes,
                &new_birth_program_attributes,
                &program_commitment,
                &program_randomness,
                &local_data_root,
                &inner_snark_id,
            );

            let outer_snark_parameters = match &parameters.outer_snark_parameters.0 {
                Some(outer_snark_parameters) => outer_snark_parameters,
                None => return Err(DPCError::MissingOuterSnarkProvingParameters),
            };

            Components::OuterSNARK::prove(&outer_snark_parameters, circuit, rng)?
        };

        let transaction = Self::Transaction::new(
            old_serial_numbers,
            new_commitments,
            memorandum.clone(),
            ledger_digest,
            inner_snark_id,
            transaction_proof,
            program_commitment,
            local_data_root,
            value_balance,
            Network::from_network_id(network_id),
            signatures,
            new_encrypted_records,
        );

        end_timer!(exec_time);

        Ok((new_records, transaction))
    }

    fn verify(parameters: &Self::Parameters, transaction: &Self::Transaction, ledger: &L) -> Result<bool, DPCError> {
        let verify_time = start_timer!(|| "BaseDPC::verify");

        // Returns false if there are duplicate serial numbers in the transaction.
        if has_duplicates(transaction.old_serial_numbers().iter()) {
            eprintln!("Transaction contains duplicate serial numbers");
            return Ok(false);
        }

        // Returns false if there are duplicate commitments numbers in the transaction.
        if has_duplicates(transaction.new_commitments().iter()) {
            eprintln!("Transaction contains duplicate commitments");
            return Ok(false);
        }

        let ledger_time = start_timer!(|| "Ledger checks");

        // Returns false if the transaction memo previously existed in the ledger.
        if ledger.contains_memo(transaction.memorandum()) {
            eprintln!("Ledger already contains this transaction memo.");
            return Ok(false);
        }

        // Returns false if any transaction serial number previously existed in the ledger.
        for sn in transaction.old_serial_numbers() {
            if ledger.contains_sn(sn) {
                eprintln!("Ledger already contains this transaction serial number.");
                return Ok(false);
            }
        }

        // Returns false if any transaction commitment previously existed in the ledger.
        for cm in transaction.new_commitments() {
            if ledger.contains_cm(cm) {
                eprintln!("Ledger already contains this transaction commitment.");
                return Ok(false);
            }
        }

        // Returns false if the ledger digest in the transaction is invalid.
        if !ledger.validate_digest(&transaction.ledger_digest) {
            eprintln!("Ledger digest is invalid.");
            return Ok(false);
        }

        end_timer!(ledger_time);

        let signature_time = start_timer!(|| "Signature checks");

        let signature_message = &to_bytes![
            transaction.network_id(),
            transaction.ledger_digest(),
            transaction.old_serial_numbers(),
            transaction.new_commitments(),
            transaction.program_commitment(),
            transaction.local_data_root(),
            transaction.value_balance(),
            transaction.memorandum()
        ]?;

        let account_signature = &parameters.system_parameters.account_signature;
        for (pk, sig) in transaction.old_serial_numbers().iter().zip(&transaction.signatures) {
            if !Components::AccountSignature::verify(account_signature, pk, signature_message, sig)? {
                eprintln!("Signature didn't verify.");
                return Ok(false);
            }
        }

        end_timer!(signature_time);

        // Construct the ciphertext hashes

        let mut new_encrypted_record_hashes = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);
        for encrypted_record in &transaction.encrypted_records {
            let encrypted_record_hash =
                RecordEncryption::encrypted_record_hash(&parameters.system_parameters, encrypted_record)?;

            new_encrypted_record_hashes.push(encrypted_record_hash);
        }

        let inner_snark_input = InnerCircuitVerifierInput {
            system_parameters: parameters.system_parameters.clone(),
            ledger_parameters: ledger.parameters().clone(),
            ledger_digest: transaction.ledger_digest().clone(),
            old_serial_numbers: transaction.old_serial_numbers().to_vec(),
            new_commitments: transaction.new_commitments().to_vec(),
            new_encrypted_record_hashes,
            memo: transaction.memorandum().clone(),
            program_commitment: transaction.program_commitment().clone(),
            local_data_root: transaction.local_data_root().clone(),
            value_balance: transaction.value_balance(),
            network_id: transaction.network_id(),
        };

        let inner_snark_vk: <<Components as BaseDPCComponents>::InnerSNARK as SNARK>::VerificationParameters =
            parameters.inner_snark_parameters.1.clone().into();

        let inner_snark_id = Components::InnerSNARKVerificationKeyCRH::hash(
            &parameters.system_parameters.inner_snark_verification_key_crh,
            &to_bytes![inner_snark_vk]?,
        )?;

        let outer_snark_input = OuterCircuitVerifierInput {
            inner_snark_verifier_input: inner_snark_input,
            inner_snark_id,
        };

        if !Components::OuterSNARK::verify(
            &parameters.outer_snark_parameters.1,
            &outer_snark_input,
            &transaction.transaction_proof,
        )? {
            eprintln!("Transaction proof failed to verify.");
            return Ok(false);
        }

        end_timer!(verify_time);

        Ok(true)
    }

    /// Returns true iff all the transactions in the block are valid according to the ledger.
    fn verify_transactions(
        parameters: &Self::Parameters,
        transactions: &Vec<Self::Transaction>,
        ledger: &L,
    ) -> Result<bool, DPCError> {
        for transaction in transactions {
            if !Self::verify(parameters, transaction, ledger)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
