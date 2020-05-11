use crate::dpc::{
    base_dpc::{binding_signature::*, record_payload::PaymentRecordPayload},
    DPCScheme,
};
use snarkos_algorithms::merkle_tree::{MerkleParameters, MerklePath, MerkleTreeDigest};
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, CRH, PRF, SNARK},
    curves::{Group, ProjectiveCurve},
    dpc::{DPCComponents, Predicate, Record},
    gadgets::algorithms::{BindingSignatureGadget, CRHGadget, CommitmentGadget, SNARKVerifierGadget},
    objects::AccountScheme,
};
use snarkos_objects::{dpc::Transaction, ledger::*, Account, AccountPrivateKey, AccountPublicKey};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
    to_bytes,
};

use rand::Rng;
use std::marker::PhantomData;

pub mod binding_signature;

pub mod predicate;
use self::predicate::*;

pub mod record;
use self::record::*;

pub mod transaction;
use self::transaction::*;

pub mod inner_circuit;
use self::inner_circuit::*;

pub mod inner_circuit_gadget;
pub use self::inner_circuit_gadget::*;

pub mod inner_circuit_verifier_input;
use self::inner_circuit_verifier_input::*;

pub mod payment_circuit;
use self::payment_circuit::*;

pub mod outer_circuit;
use self::outer_circuit::*;

pub mod outer_circuit_gadget;
pub use self::outer_circuit_gadget::*;

pub mod outer_circuit_verifier_input;
use self::outer_circuit_verifier_input::*;

pub mod parameters;
use self::parameters::*;

pub mod record_payload;

pub mod instantiated;

#[cfg(test)]
mod test;

///////////////////////////////////////////////////////////////////////////////

/// Trait that stores all information about the components of a Plain DPC
/// scheme. Simplifies the interface of Plain DPC by wrapping all these into
/// one.
pub trait BaseDPCComponents: DPCComponents {
    /// Ledger digest type.
    type MerkleParameters: MerkleParameters;
    type MerkleHashGadget: CRHGadget<<Self::MerkleParameters as MerkleParameters>::H, Self::InnerField>;

    /// Commitment scheme for committing to a record value
    type ValueCommitment: CommitmentScheme;
    type ValueCommitmentGadget: CommitmentGadget<Self::ValueCommitment, Self::InnerField>;

    /// Gadget for verifying the binding signature
    type BindingSignatureGroup: Group + ProjectiveCurve;
    type BindingSignatureGadget: BindingSignatureGadget<
        Self::ValueCommitment,
        Self::InnerField,
        Self::BindingSignatureGroup,
    >;

    /// SNARK for non-proof-verification checks
    type InnerSNARK: SNARK<
        Circuit = InnerCircuit<Self>,
        AssignedCircuit = InnerCircuit<Self>,
        VerifierInput = InnerCircuitVerifierInput<Self>,
    >;

    /// SNARK for proof-verification checks
    type OuterSNARK: SNARK<
        Circuit = OuterCircuit<Self>,
        AssignedCircuit = OuterCircuit<Self>,
        VerifierInput = OuterCircuitVerifierInput<Self>,
    >;

    /// SNARK for a "dummy predicate" that does nothing with its input.
    type PredicateSNARK: SNARK<
        Circuit = PaymentCircuit<Self>,
        AssignedCircuit = PaymentCircuit<Self>,
        VerifierInput = PaymentPredicateLocalData<Self>,
    >;

    /// SNARK Verifier gadget for the "dummy predicate" that does nothing with its input.
    type PredicateSNARKGadget: SNARKVerifierGadget<Self::PredicateSNARK, Self::OuterField>;
}

///////////////////////////////////////////////////////////////////////////////

pub struct DPC<Components: BaseDPCComponents> {
    _components: PhantomData<Components>,
}

/// Returned by `PlainDPC::execute_helper`. Stores data required to produce the
/// final transaction after `execute_helper` has created old serial numbers and
/// ledger witnesses, and new records and commitments. For convenience, it also
/// stores references to existing information like old records and secret keys.
pub(crate) struct ExecuteContext<'a, Components: BaseDPCComponents> {
    circuit_parameters: &'a CircuitParameters<Components>,
    ledger_digest: MerkleTreeDigest<Components::MerkleParameters>,

    // Old record stuff
    old_account_private_keys: &'a [AccountPrivateKey<Components>],
    old_records: &'a [DPCRecord<Components>],
    old_witnesses: Vec<MerklePath<Components::MerkleParameters>>,
    old_serial_numbers: Vec<<Components::Signature as SignatureScheme>::PublicKey>,
    old_randomizers: Vec<Vec<u8>>,

    // New record stuff
    new_records: Vec<DPCRecord<Components>>,
    new_sn_nonce_randomness: Vec<[u8; 32]>,
    new_commitments: Vec<<Components::RecordCommitment as CommitmentScheme>::Output>,

    // Predicate and local data commitment and randomness
    predicate_commitment: <Components::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    predicate_randomness: <Components::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,

    local_data_commitment: <Components::LocalDataCommitment as CommitmentScheme>::Output,
    local_data_randomness: <Components::LocalDataCommitment as CommitmentScheme>::Randomness,

    // Value Balance
    value_balance: i64,
}

impl<Components: BaseDPCComponents> ExecuteContext<'_, Components> {
    fn into_local_data(&self) -> LocalData<Components> {
        LocalData {
            circuit_parameters: self.circuit_parameters.clone(),

            old_records: self.old_records.to_vec(),
            old_serial_numbers: self.old_serial_numbers.to_vec(),

            new_records: self.new_records.to_vec(),

            local_data_commitment: self.local_data_commitment.clone(),
            local_data_randomness: self.local_data_randomness.clone(),
        }
    }
}

/// Stores local data required to produce predicate proofs.
pub struct LocalData<Components: BaseDPCComponents> {
    pub circuit_parameters: CircuitParameters<Components>,

    // Old records and serial numbers
    pub old_records: Vec<DPCRecord<Components>>,
    pub old_serial_numbers: Vec<<Components::Signature as SignatureScheme>::PublicKey>,

    // New records
    pub new_records: Vec<DPCRecord<Components>>,

    // Commitment to the above information.
    pub local_data_commitment: <Components::LocalDataCommitment as CommitmentScheme>::Output,
    pub local_data_randomness: <Components::LocalDataCommitment as CommitmentScheme>::Randomness,
}

///////////////////////////////////////////////////////////////////////////////

impl<Components: BaseDPCComponents> DPC<Components> {
    pub fn generate_circuit_parameters<R: Rng>(rng: &mut R) -> Result<CircuitParameters<Components>, DPCError> {
        let time = start_timer!(|| "Account commitment scheme setup");
        let account_commitment = Components::AccountCommitment::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Record commitment scheme setup");
        let rec_comm_pp = Components::RecordCommitment::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Verification Key Commitment setup");
        let pred_vk_comm_pp = Components::PredicateVerificationKeyCommitment::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Local Data Commitment setup");
        let local_data_comm_pp = Components::LocalDataCommitment::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Value Commitment setup");
        let value_comm_pp = Components::ValueCommitment::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Serial Nonce CRH setup");
        let sn_nonce_crh_pp = Components::SerialNumberNonceCRH::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Verification Key CRH setup");
        let pred_vk_crh_pp = Components::PredicateVerificationKeyHash::setup(rng);
        end_timer!(time);

        let time = start_timer!(|| "Signature setup");
        let sig_pp = Components::Signature::setup(rng)?;
        end_timer!(time);

        let comm_crh_sig_pp = CircuitParameters {
            account_commitment,
            record_commitment: rec_comm_pp,
            predicate_verification_key_commitment: pred_vk_comm_pp,
            local_data_commitment: local_data_comm_pp,
            value_commitment: value_comm_pp,

            serial_number_nonce: sn_nonce_crh_pp,
            predicate_verification_key_hash: pred_vk_crh_pp,

            signature: sig_pp,
        };

        Ok(comm_crh_sig_pp)
    }

    pub fn generate_pred_nizk_parameters<R: Rng>(
        comm_crh_sig_pp: &CircuitParameters<Components>,
        rng: &mut R,
    ) -> Result<PredicateSNARKParameters<Components>, DPCError> {
        let (pk, pvk) = Components::PredicateSNARK::setup(PaymentCircuit::blank(comm_crh_sig_pp), rng)?;
        let proof = Components::PredicateSNARK::prove(&pk, PaymentCircuit::blank(comm_crh_sig_pp), rng)?;

        Ok(PredicateSNARKParameters {
            proving_key: pk,
            verification_key: pvk.into(),
            proof,
        })
    }

    pub fn generate_sn(
        params: &CircuitParameters<Components>,
        record: &DPCRecord<Components>,
        account_private_key: &AccountPrivateKey<Components>,
    ) -> Result<(<Components::Signature as SignatureScheme>::PublicKey, Vec<u8>), DPCError> {
        let sn_time = start_timer!(|| "Generate serial number");
        let sk_prf = &account_private_key.sk_prf;
        let sn_nonce = to_bytes!(record.serial_number_nonce())?;
        // Compute the serial number.
        let prf_input = FromBytes::read(sn_nonce.as_slice())?;
        let prf_seed = FromBytes::read(to_bytes!(sk_prf)?.as_slice())?;
        let sig_and_pk_randomizer = to_bytes![Components::PRF::evaluate(&prf_seed, &prf_input)?]?;

        let sn = Components::Signature::randomize_public_key(
            &params.signature,
            &account_private_key.pk_sig,
            &sig_and_pk_randomizer,
        )?;
        end_timer!(sn_time);
        Ok((sn, sig_and_pk_randomizer))
    }

    pub fn generate_record<R: Rng>(
        parameters: &CircuitParameters<Components>,
        sn_nonce: &<Components::SerialNumberNonceCRH as CRH>::Output,
        account_public_key: &AccountPublicKey<Components>,
        is_dummy: bool,
        payload: &PaymentRecordPayload,
        birth_predicate: &DPCPredicate<Components>,
        death_predicate: &DPCPredicate<Components>,
        rng: &mut R,
    ) -> Result<DPCRecord<Components>, DPCError> {
        let record_time = start_timer!(|| "Generate record");
        // Sample new commitment randomness.
        let commitment_randomness = <Components::RecordCommitment as CommitmentScheme>::Randomness::rand(rng);

        // Construct a record commitment.
        let birth_predicate_repr = birth_predicate.into_compact_repr();
        let death_predicate_repr = death_predicate.into_compact_repr();
        // Total = 32 + 1 + 32 + 32 + 32 + 32 = 161 bytes
        let commitment_input = to_bytes![
            account_public_key.public_key, // 256 bits = 32 bytes
            is_dummy,                      // 1 bit = 1 byte
            payload,                       // 256 bits = 32 bytes
            birth_predicate_repr,          // 256 bits = 32 bytes
            death_predicate_repr,          // 256 bits = 32 bytes
            sn_nonce                       // 256 bits = 32 bytes
        ]?;

        let commitment = Components::RecordCommitment::commit(
            &parameters.record_commitment,
            &commitment_input,
            &commitment_randomness,
        )?;

        let record = DPCRecord {
            account_public_key: account_public_key.clone(),
            is_dummy,
            payload: payload.clone(),
            birth_predicate_repr,
            death_predicate_repr,
            serial_number_nonce: sn_nonce.clone(),
            commitment,
            commitment_randomness,
            _components: PhantomData,
        };
        end_timer!(record_time);
        Ok(record)
    }

    pub(crate) fn execute_helper<'a, L, R: Rng>(
        parameters: &'a CircuitParameters<Components>,

        old_records: &'a [<Self as DPCScheme<L>>::Record],
        old_account_private_keys: &'a [AccountPrivateKey<Components>],

        new_account_public_keys: &[AccountPublicKey<Components>],
        new_is_dummy_flags: &[bool],
        new_payloads: &[<Self as DPCScheme<L>>::Payload],
        new_birth_predicates: &[<Self as DPCScheme<L>>::Predicate],
        new_death_predicates: &[<Self as DPCScheme<L>>::Predicate],

        memo: &[u8; 32],
        auxiliary: &[u8; 32],

        ledger: &L,
        rng: &mut R,
    ) -> Result<ExecuteContext<'a, Components>, DPCError>
    where
        L: Ledger<
            Parameters = Components::MerkleParameters,
            Commitment = <Components::RecordCommitment as CommitmentScheme>::Output,
            SerialNumber = <Components::Signature as SignatureScheme>::PublicKey,
        >,
    {
        assert_eq!(Components::NUM_INPUT_RECORDS, old_records.len());
        assert_eq!(Components::NUM_INPUT_RECORDS, old_account_private_keys.len());

        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_account_public_keys.len());
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_is_dummy_flags.len());
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_payloads.len());
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_birth_predicates.len());
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_death_predicates.len());

        let mut old_witnesses = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        let mut old_serial_numbers = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        let mut old_randomizers = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        let mut joint_serial_numbers = Vec::new();
        let mut old_death_pred_hashes = Vec::new();

        let mut value_balance: i64 = 0;

        // Compute the ledger membership witness and serial number from the old records.
        for (i, record) in old_records.iter().enumerate() {
            let input_record_time = start_timer!(|| format!("Process input record {}", i));

            if record.is_dummy() {
                old_witnesses.push(MerklePath::default());
            } else {
                let comm = &record.commitment();
                let witness = ledger.prove_cm(comm)?;
                old_witnesses.push(witness);

                value_balance += record.payload.balance as i64;
            }

            let (sn, randomizer) = Self::generate_sn(&parameters, record, &old_account_private_keys[i])?;
            joint_serial_numbers.extend_from_slice(&to_bytes![sn]?);
            old_serial_numbers.push(sn);
            old_randomizers.push(randomizer);
            old_death_pred_hashes.push(record.death_predicate_repr().to_vec());

            end_timer!(input_record_time);
        }

        let mut new_records = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);
        let mut new_commitments = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);
        let mut new_sn_nonce_randomness = Vec::with_capacity(Components::NUM_OUTPUT_RECORDS);
        let mut new_birth_pred_hashes = Vec::new();

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
                &new_account_public_keys[j],
                new_is_dummy_flags[j],
                &new_payloads[j],
                &new_birth_predicates[j],
                &new_death_predicates[j],
                rng,
            )?;

            if !record.is_dummy {
                value_balance -= record.payload.balance as i64;
            }

            new_commitments.push(record.commitment.clone());
            new_sn_nonce_randomness.push(sn_randomness);
            new_birth_pred_hashes.push(record.birth_predicate_repr().to_vec());
            new_records.push(record);

            end_timer!(output_record_time);
        }

        let local_data_comm_timer = start_timer!(|| "Compute predicate input commitment");
        let mut predicate_input = Vec::new();
        for i in 0..Components::NUM_INPUT_RECORDS {
            let record = &old_records[i];
            let bytes = to_bytes![
                record.commitment(),
                record.account_public_key(),
                record.is_dummy(),
                record.payload(),
                record.birth_predicate_repr(),
                record.death_predicate_repr(),
                old_serial_numbers[i]
            ]?;
            predicate_input.extend_from_slice(&bytes);
        }

        for j in 0..Components::NUM_OUTPUT_RECORDS {
            let record = &new_records[j];
            let bytes = to_bytes![
                record.commitment(),
                record.account_public_key(),
                record.is_dummy(),
                record.payload(),
                record.birth_predicate_repr(),
                record.death_predicate_repr()
            ]?;
            predicate_input.extend_from_slice(&bytes);
        }
        predicate_input.extend_from_slice(memo);
        predicate_input.extend_from_slice(auxiliary);

        let local_data_rand = <Components::LocalDataCommitment as CommitmentScheme>::Randomness::rand(rng);
        let local_data_comm = Components::LocalDataCommitment::commit(
            &parameters.local_data_commitment,
            &predicate_input,
            &local_data_rand,
        )?;
        end_timer!(local_data_comm_timer);

        let pred_hash_comm_timer = start_timer!(|| "Compute predicate commitment");
        let (predicate_comm, predicate_rand) = {
            let mut input = Vec::new();
            for hash in old_death_pred_hashes {
                input.extend_from_slice(&hash);
            }

            for hash in new_birth_pred_hashes {
                input.extend_from_slice(&hash);
            }
            let predicate_rand =
                <Components::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness::rand(rng);
            let predicate_comm = Components::PredicateVerificationKeyCommitment::commit(
                &parameters.predicate_verification_key_commitment,
                &input,
                &predicate_rand,
            )?;
            (predicate_comm, predicate_rand)
        };
        end_timer!(pred_hash_comm_timer);

        let ledger_digest = ledger.digest().expect("could not get digest");

        let context = ExecuteContext {
            circuit_parameters: parameters,
            ledger_digest,

            old_records,
            old_witnesses,
            old_account_private_keys,
            old_serial_numbers,
            old_randomizers,

            new_records,
            new_sn_nonce_randomness,
            new_commitments,

            predicate_commitment: predicate_comm,
            predicate_randomness: predicate_rand,
            local_data_commitment: local_data_comm,
            local_data_randomness: local_data_rand,

            value_balance,
        };
        Ok(context)
    }
}

impl<Components: BaseDPCComponents, L: Ledger> DPCScheme<L> for DPC<Components>
where
    L: Ledger<
        Parameters = Components::MerkleParameters,
        Commitment = <Components::RecordCommitment as CommitmentScheme>::Output,
        SerialNumber = <Components::Signature as SignatureScheme>::PublicKey,
    >,
{
    type Account = Account<Components>;
    type Auxiliary = [u8; 32];
    type LocalData = LocalData<Components>;
    type Metadata = [u8; 32];
    type Parameters = PublicParameters<Components>;
    type Payload = <Self::Record as Record>::Payload;
    type Predicate = DPCPredicate<Components>;
    type PrivatePredInput = PrivatePredicateInput<Components>;
    type Record = DPCRecord<Components>;
    type Transaction = DPCTransaction<Components>;

    fn setup<R: Rng>(
        ledger_parameters: &Components::MerkleParameters,
        rng: &mut R,
    ) -> Result<Self::Parameters, DPCError> {
        let setup_time = start_timer!(|| "BaseDPC::Setup");
        let circuit_parameters = Self::generate_circuit_parameters(rng)?;

        let predicate_snark_setup_time = start_timer!(|| "Dummy Predicate SNARK Setup");
        let predicate_snark_parameters = Self::generate_pred_nizk_parameters(&circuit_parameters, rng)?;
        end_timer!(predicate_snark_setup_time);

        let private_pred_input = PrivatePredicateInput {
            verification_key: predicate_snark_parameters.verification_key.clone(),
            proof: predicate_snark_parameters.proof.clone(),
            value_commitment: <Components::ValueCommitment as CommitmentScheme>::Output::default(),
            value_commitment_randomness: <Components::ValueCommitment as CommitmentScheme>::Randomness::default(),
        };

        let snark_setup_time = start_timer!(|| "Execute Inner SNARK Setup");
        let inner_snark_parameters =
            Components::InnerSNARK::setup(InnerCircuit::blank(&circuit_parameters, ledger_parameters), rng)?;
        end_timer!(snark_setup_time);

        let snark_setup_time = start_timer!(|| "Execute Outer SNARK Setup");
        let outer_snark_parameters =
            Components::OuterSNARK::setup(OuterCircuit::blank(&circuit_parameters, &private_pred_input), rng)?;
        end_timer!(snark_setup_time);
        end_timer!(setup_time);

        let inner_snark_parameters = (Some(inner_snark_parameters.0), inner_snark_parameters.1);
        let outer_snark_parameters = (Some(outer_snark_parameters.0), outer_snark_parameters.1);

        Ok(PublicParameters {
            circuit_parameters,
            predicate_snark_parameters,
            inner_snark_parameters,
            outer_snark_parameters,
        })
    }

    fn create_account<R: Rng>(
        parameters: &Self::Parameters,
        metadata: &Self::Metadata,
        rng: &mut R,
    ) -> Result<Self::Account, DPCError> {
        let time = start_timer!(|| "BaseDPC::create_account");

        let signature_parameters = &parameters.circuit_parameters.signature;
        let commitment_parameters = &parameters.circuit_parameters.account_commitment;
        let account = Account::new(signature_parameters, commitment_parameters, metadata, None, rng)?;

        end_timer!(time);

        Ok(account)
    }

    fn execute<R: Rng>(
        parameters: &Self::Parameters,
        old_records: &[Self::Record],
        old_account_private_keys: &[<Self::Account as AccountScheme>::AccountPrivateKey],
        mut old_death_pred_proof_generator: impl FnMut(&Self::LocalData) -> Result<Vec<Self::PrivatePredInput>, DPCError>,

        new_account_public_keys: &[<Self::Account as AccountScheme>::AccountPublicKey],
        new_is_dummy_flags: &[bool],
        new_payloads: &[Self::Payload],
        new_birth_predicates: &[Self::Predicate],
        new_death_predicates: &[Self::Predicate],
        mut new_birth_pred_proof_generator: impl FnMut(&Self::LocalData) -> Result<Vec<Self::PrivatePredInput>, DPCError>,

        auxiliary: &Self::Auxiliary,
        memorandum: &<Self::Transaction as Transaction>::Memorandum,
        ledger: &L,
        rng: &mut R,
    ) -> Result<(Vec<Self::Record>, Self::Transaction), DPCError> {
        let exec_time = start_timer!(|| "BaseDPC::execute");
        let context = Self::execute_helper(
            &parameters.circuit_parameters,
            old_records,
            old_account_private_keys,
            new_account_public_keys,
            new_is_dummy_flags,
            new_payloads,
            new_birth_predicates,
            new_death_predicates,
            memorandum,
            auxiliary,
            ledger,
            rng,
        )?;

        let local_data = context.into_local_data();
        let old_death_pred_attributes = old_death_pred_proof_generator(&local_data)?;
        let new_birth_pred_attributes = new_birth_pred_proof_generator(&local_data)?;

        let ExecuteContext {
            circuit_parameters,
            ledger_digest,

            old_records,
            old_witnesses,
            old_account_private_keys,
            old_serial_numbers,
            old_randomizers,

            new_records,
            new_sn_nonce_randomness,
            new_commitments,
            predicate_commitment,
            predicate_randomness,
            local_data_commitment,
            local_data_randomness,
            value_balance,
        } = context;

        // Generate binding signature

        let mut old_value_commits = vec![];
        let mut old_value_commit_randomness = vec![];
        let mut new_value_commits = vec![];
        let mut new_value_commit_randomness = vec![];

        for death_pred_attr in &old_death_pred_attributes {
            let mut commitment = [0u8; 32];
            let mut randomness = [0u8; 32];

            death_pred_attr.value_commitment.write(&mut commitment[..])?;
            death_pred_attr.value_commitment_randomness.write(&mut randomness[..])?;

            old_value_commits.push(commitment);
            old_value_commit_randomness.push(randomness);
        }

        for birth_pred_attr in &new_birth_pred_attributes {
            let mut commitment = [0u8; 32];
            let mut randomness = [0u8; 32];

            birth_pred_attr.value_commitment.write(&mut commitment[..])?;
            birth_pred_attr.value_commitment_randomness.write(&mut randomness[..])?;

            new_value_commits.push(commitment);
            new_value_commit_randomness.push(randomness);
        }

        let sighash = to_bytes![local_data_commitment]?;

        let binding_signature =
            create_binding_signature::<Components::ValueCommitment, Components::BindingSignatureGroup, _>(
                &circuit_parameters.value_commitment,
                &old_value_commits,
                &new_value_commits,
                &old_value_commit_randomness,
                &new_value_commit_randomness,
                value_balance,
                &sighash,
                rng,
            )?;

        let inner_proof = {
            let circuit = InnerCircuit::new(
                &parameters.circuit_parameters,
                ledger.parameters(),
                &ledger_digest,
                old_records,
                &old_witnesses,
                old_account_private_keys,
                &old_serial_numbers,
                &new_records,
                &new_sn_nonce_randomness,
                &new_commitments,
                &predicate_commitment,
                &predicate_randomness,
                &local_data_commitment,
                &local_data_randomness,
                memorandum,
                auxiliary,
                &old_value_commits,
                &new_value_commits,
                value_balance,
                &binding_signature,
            );

            let inner_snark_parameters = match &parameters.inner_snark_parameters.0 {
                Some(inner_snark_parameters) => inner_snark_parameters,
                None => return Err(DPCError::MissingInnerSnarkProvingParameters),
            };

            Components::InnerSNARK::prove(&inner_snark_parameters, circuit, rng)?
        };

        let outer_proof = {
            let circuit = OuterCircuit::new(
                &parameters.circuit_parameters,
                old_death_pred_attributes.as_slice(),
                new_birth_pred_attributes.as_slice(),
                &predicate_commitment,
                &predicate_randomness,
                &local_data_commitment,
            );

            let outer_snark_parameters = match &parameters.outer_snark_parameters.0 {
                Some(outer_snark_parameters) => outer_snark_parameters,
                None => return Err(DPCError::MissingOuterSnarkProvingParameters),
            };

            Components::OuterSNARK::prove(&outer_snark_parameters, circuit, rng)?
        };

        let signature_message = to_bytes![
            old_serial_numbers,
            new_commitments,
            memorandum,
            ledger_digest,
            inner_proof,
            outer_proof
        ]?;

        let mut signatures = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        for i in 0..Components::NUM_INPUT_RECORDS {
            let sig_time = start_timer!(|| format!("Sign and randomize Tx contents {}", i));

            let sk_sig = &old_account_private_keys[i].sk_sig;
            let randomizer = &old_randomizers[i];
            // Sign transaction message
            let signature =
                Components::Signature::sign(&circuit_parameters.signature, sk_sig, &signature_message, rng)?;
            let randomized_signature =
                Components::Signature::randomize_signature(&circuit_parameters.signature, &signature, randomizer)?;
            signatures.push(randomized_signature);

            end_timer!(sig_time);
        }

        let transaction = Self::Transaction::new(
            old_serial_numbers,
            new_commitments,
            memorandum.clone(),
            ledger_digest,
            inner_proof,
            outer_proof,
            predicate_commitment,
            local_data_commitment,
            value_balance,
            signatures,
        );

        end_timer!(exec_time);
        Ok((new_records, transaction))
    }

    fn verify(parameters: &Self::Parameters, transaction: &Self::Transaction, ledger: &L) -> Result<bool, DPCError> {
        let verify_time = start_timer!(|| "BaseDPC::verify");
        let ledger_time = start_timer!(|| "Ledger checks");
        for sn in transaction.old_serial_numbers() {
            if ledger.contains_sn(sn) {
                eprintln!("Ledger contains this serial number already.");
                return Ok(false);
            }
        }

        // This is quadratic, but doesn't really matter.
        for (i, sn_i) in transaction.old_serial_numbers().iter().enumerate() {
            for (j, sn_j) in transaction.old_serial_numbers().iter().enumerate() {
                if i != j && sn_i == sn_j {
                    eprintln!("Transaction contains duplicate serial numbers");
                    return Ok(false);
                }
            }
        }

        // Check that the record commitment digest is valid.
        if !ledger.validate_digest(&transaction.stuff.digest) {
            eprintln!("Ledger digest is invalid.");
            return Ok(false);
        }
        end_timer!(ledger_time);

        let input = InnerCircuitVerifierInput {
            circuit_parameters: parameters.circuit_parameters.clone(),
            ledger_parameters: ledger.parameters().clone(),
            ledger_digest: transaction.stuff.digest.clone(),
            old_serial_numbers: transaction.old_serial_numbers().to_vec(),
            new_commitments: transaction.new_commitments().to_vec(),
            memo: transaction.memorandum().clone(),
            predicate_commitment: transaction.stuff.predicate_commitment.clone(),
            local_data_commitment: transaction.stuff.local_data_commitment.clone(),
            value_balance: transaction.stuff.value_balance,
        };
        if !Components::InnerSNARK::verify(
            &parameters.inner_snark_parameters.1,
            &input,
            &transaction.stuff.inner_proof,
        )? {
            eprintln!("Core NIZK didn't verify.");
            return Ok(false);
        };

        let input = OuterCircuitVerifierInput {
            circuit_parameters: parameters.circuit_parameters.clone(),
            predicate_commitment: transaction.stuff.predicate_commitment.clone(),
            local_data_commitment: transaction.stuff.local_data_commitment.clone(),
        };

        if !Components::OuterSNARK::verify(
            &parameters.outer_snark_parameters.1,
            &input,
            &transaction.stuff.predicate_proof,
        )? {
            eprintln!("Predicate check NIZK didn't verify.");
            return Ok(false);
        }

        let signature_message = &to_bytes![
            transaction.old_serial_numbers(),
            transaction.new_commitments(),
            transaction.memorandum(),
            transaction.stuff.digest,
            transaction.stuff.inner_proof,
            transaction.stuff.predicate_proof
        ]?;

        let sig_time = start_timer!(|| "Signature verification (in parallel)");
        let sig_pp = &parameters.circuit_parameters.signature;
        for (pk, sig) in transaction
            .old_serial_numbers()
            .iter()
            .zip(&transaction.stuff.signatures)
        {
            if !Components::Signature::verify(sig_pp, pk, signature_message, sig)? {
                eprintln!("Signature didn't verify.");
                return Ok(false);
            }
        }
        end_timer!(sig_time);

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
