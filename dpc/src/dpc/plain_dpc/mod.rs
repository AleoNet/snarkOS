use crate::Error;
use rand::Rng;
use std::marker::PhantomData;

use crate::{
    dpc::{
        AddressKeyPair, DPCScheme, Predicate,
        Record, Transaction,
    },
    ledger::*,
};
use snarkos_algorithms::merkle_tree::{MerkleParameters, MerklePath, MerkleTreeDigest};
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH, PRF, SNARK},
    curves::PrimeField,
    gadgets::algorithms::{
        CommitmentGadget, CRHGadget, PRFGadget,
        SNARKVerifierGadget,
    },
};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
    to_bytes,
};

pub mod address;
use self::address::*;

pub mod predicate;
use self::predicate::*;

pub mod record;
use self::record::*;

pub mod transaction;
use self::transaction::*;

pub mod core_checks_circuit;
use self::core_checks_circuit::*;

pub mod proof_check_circuit;
use self::proof_check_circuit::*;

pub mod predicate_circuit;
use self::predicate_circuit::*;

pub mod parameters;
use self::parameters::*;

pub mod instantiated;

#[cfg(test)]
mod test;

///////////////////////////////////////////////////////////////////////////////

/// Trait that stores all information about the components of a Plain DPC
/// scheme. Simplifies the interface of Plain DPC by wrapping all these into
/// one.
pub trait PlainDPCComponents: 'static + Sized {
    const NUM_INPUT_RECORDS: usize;
    const NUM_OUTPUT_RECORDS: usize;

    type CoreCheckF: PrimeField;
    type ProofCheckF: PrimeField;

    // Commitment scheme for address contents. Invoked only over `Self::CoreCheckF`.
    type AddrC: CommitmentScheme;
    type AddrCGadget: CommitmentGadget<Self::AddrC, Self::CoreCheckF>;

    // Commitment scheme for record contents. Invoked only over `Self::CoreCheckF`.
    type RecC: CommitmentScheme;
    type RecCGadget: CommitmentGadget<Self::RecC, Self::CoreCheckF>;

    // Ledger digest type.
    type MerkleParameters: MerkleParameters;
    type MerkleTree_HGadget: CRHGadget<<Self::MerkleParameters as MerkleParameters>::H, Self::CoreCheckF>;

    // CRH for computing the serial number nonce. Invoked only over `Self::CoreCheckF`.
    type SnNonceH: CRH;
    type SnNonceHGadget: CRHGadget<Self::SnNonceH, Self::CoreCheckF>;

    // CRH for hashes of birth and death verification keys.
    // This is invoked only on the larger curve.
    type PredVkH: CRH;
    type PredVkHGadget: CRHGadget<Self::PredVkH, Self::ProofCheckF>;

    // Commitment scheme for committing to hashes of birth and death verification
    // keys
    type PredVkComm: CommitmentScheme;
    // Used to commit to hashes of vkeys on the smaller curve and to decommit hashes
    // of vkeys on the larger curve
    type PredVkCommGadget: CommitmentGadget<Self::PredVkComm, Self::CoreCheckF>
        + CommitmentGadget<Self::PredVkComm, Self::ProofCheckF>;

    // Commitment scheme for committing to predicate input. Invoked inside
    // `Self::MainN` and every predicate SNARK.
    type LocalDataComm: CommitmentScheme;
    type LocalDataCommGadget: CommitmentGadget<Self::LocalDataComm, Self::CoreCheckF>;

    // SNARK for non-proof-verification checks
    type MainNIZK: SNARK<
        Circuit = CoreChecksCircuit<Self>,
        AssignedCircuit = CoreChecksCircuit<Self>,
        VerifierInput = CoreChecksVerifierInput<Self>,
    >;

    // SNARK for proof-verification checks
    type ProofCheckNIZK: SNARK<
        Circuit = ProofCheckCircuit<Self>,
        AssignedCircuit = ProofCheckCircuit<Self>,
        VerifierInput = ProofCheckVerifierInput<Self>,
    >;

    // SNARK for a "dummy predicate" that does nothing with its input.
    type PredicateNIZK: SNARK<
        Circuit = EmptyPredicateCircuit<Self>,
        AssignedCircuit = EmptyPredicateCircuit<Self>,
        VerifierInput = PredicateLocalData<Self>,
    >;

    // SNARK Verifier gadget for the "dummy predicate" that does nothing with its
    // input.
    type PredicateNIZKGadget: SNARKVerifierGadget<Self::PredicateNIZK, Self::ProofCheckF>;

    // PRF for computing serial numbers. Invoked only over `Self::CoreCheckF`.
    type P: PRF;
    type PGadget: PRFGadget<Self::P, Self::CoreCheckF>;
}

///////////////////////////////////////////////////////////////////////////////

pub struct DPC<Components: PlainDPCComponents> {
    _components: PhantomData<Components>,
}

/// Returned by `PlainDPC::execute_helper`. Stores data required to produce the
/// final transaction after `execute_helper` has created old serial numbers and
/// ledger witnesses, and new records and commitments. For convenience, it also
/// stores references to existing information like old records and secret keys.
pub(crate) struct ExecuteContext<'a, Components: PlainDPCComponents> {
    comm_and_crh_pp: &'a CommAndCRHPublicParameters<Components>,
    ledger_digest:   MerkleTreeDigest<Components::MerkleParameters>,

    // Old record stuff
    old_address_secret_keys: &'a [AddressSecretKey<Components>],
    old_records:             &'a [DPCRecord<Components>],
    old_witnesses:           Vec<MerklePath<Components::MerkleParameters>>,
    old_serial_numbers:      Vec<<Components::P as PRF>::Output>,

    // New record stuff
    new_records:             Vec<DPCRecord<Components>>,
    new_sn_nonce_randomness: Vec<[u8; 32]>,
    new_commitments:         Vec<<Components::RecC as CommitmentScheme>::Output>,

    // Predicate and local data commitment and randomness
    predicate_comm: <Components::PredVkComm as CommitmentScheme>::Output,
    predicate_rand: <Components::PredVkComm as CommitmentScheme>::Randomness,

    local_data_comm: <Components::LocalDataComm as CommitmentScheme>::Output,
    local_data_rand: <Components::LocalDataComm as CommitmentScheme>::Randomness,
}

impl<Components: PlainDPCComponents> ExecuteContext<'_, Components> {
    fn into_local_data(&self) -> LocalData<Components> {
        LocalData {
            comm_and_crh_pp: self.comm_and_crh_pp.clone(),

            old_records:        self.old_records.to_vec(),
            old_serial_numbers: self.old_serial_numbers.to_vec(),

            new_records: self.new_records.to_vec(),

            local_data_comm: self.local_data_comm.clone(),
            local_data_rand: self.local_data_rand.clone(),
        }
    }
}

/// Stores local data required to produce predicate proofs.
pub struct LocalData<Components: PlainDPCComponents> {
    pub comm_and_crh_pp: CommAndCRHPublicParameters<Components>,

    // Old records and serial numbers
    pub old_records:        Vec<DPCRecord<Components>>,
    pub old_serial_numbers: Vec<<Components::P as PRF>::Output>,

    // New records
    pub new_records: Vec<DPCRecord<Components>>,

    // Commitment to the above information.
    pub local_data_comm: <Components::LocalDataComm as CommitmentScheme>::Output,
    pub local_data_rand: <Components::LocalDataComm as CommitmentScheme>::Randomness,
}

///////////////////////////////////////////////////////////////////////////////

impl<Components: PlainDPCComponents> DPC<Components> {
    pub fn generate_comm_and_crh_parameters<R: Rng>(
        rng: &mut R,
    ) -> Result<CommAndCRHPublicParameters<Components>, Error> {
        let time = start_timer!(|| "Address commitment scheme setup");
        let addr_comm_pp = Components::AddrC::setup(rng)?;
        end_timer!(time);

        let time = start_timer!(|| "Record commitment scheme setup");
        let rec_comm_pp = Components::RecC::setup(rng)?;
        end_timer!(time);

        let time = start_timer!(|| "Verification Key Commitment setup");
        let pred_vk_comm_pp = Components::PredVkComm::setup(rng)?;
        end_timer!(time);

        let time = start_timer!(|| "Local Data Commitment setup");
        let local_data_comm_pp = Components::LocalDataComm::setup(rng)?;
        end_timer!(time);

        let time = start_timer!(|| "Serial Nonce CRH setup");
        let sn_nonce_crh_pp = Components::SnNonceH::setup(rng)?;
        end_timer!(time);

        let time = start_timer!(|| "Verification Key CRH setup");
        let pred_vk_crh_pp = Components::PredVkH::setup(rng)?;
        end_timer!(time);

        let comm_and_crh_pp = CommAndCRHPublicParameters {
            addr_comm_pp,
            rec_comm_pp,
            pred_vk_comm_pp,
            local_data_comm_pp,

            sn_nonce_crh_pp,
            pred_vk_crh_pp,
        };
        Ok(comm_and_crh_pp)
    }

    pub fn generate_pred_nizk_parameters<R: Rng>(
        comm_and_crh_pp: &CommAndCRHPublicParameters<Components>,
        rng: &mut R,
    ) -> Result<PredNIZKParameters<Components>, Error> {
        let (pk, pvk) =
            Components::PredicateNIZK::setup(EmptyPredicateCircuit::blank(comm_and_crh_pp), rng)?;

        let proof = Components::PredicateNIZK::prove(
            &pk,
            EmptyPredicateCircuit::blank(comm_and_crh_pp),
            rng,
        )?;

        Ok(PredNIZKParameters {
            pk,
            vk: pvk.into(),
            proof,
        })
    }

    pub fn generate_sn(
        record: &DPCRecord<Components>,
        address_secret_key: &AddressSecretKey<Components>,
    ) -> Result<<Components::P as PRF>::Output, Error> {
        let sn_time = start_timer!(|| "Generate serial number");
        let sk_prf = &address_secret_key.sk_prf;
        let sn_nonce = to_bytes!(record.serial_number_nonce())?;
        // Compute the serial number.
        let prf_input = FromBytes::read(sn_nonce.as_slice())?;
        let prf_seed = FromBytes::read(to_bytes!(sk_prf)?.as_slice())?;
        let sn = Components::P::evaluate(&prf_seed, &prf_input)?;
        end_timer!(sn_time);
        Ok(sn)
    }

    pub fn generate_record<R: Rng>(
        parameters: &CommAndCRHPublicParameters<Components>,
        sn_nonce: &<Components::SnNonceH as CRH>::Output,
        address_public_key: &AddressPublicKey<Components>,
        is_dummy: bool,
        payload: &[u8; 32],
        birth_predicate: &DPCPredicate<Components>,
        death_predicate: &DPCPredicate<Components>,
        rng: &mut R,
    ) -> Result<DPCRecord<Components>, Error> {
        let record_time = start_timer!(|| "Generate record");
        // Sample new commitment randomness.
        let commitment_randomness = <Components::RecC as CommitmentScheme>::Randomness::rand(rng);

        // Construct a record commitment.
        let birth_predicate_repr = birth_predicate.into_compact_repr();
        let death_predicate_repr = death_predicate.into_compact_repr();
        // Total = 32 + 1 + 32 + 32 + 32 + 32 = 161 bytes
        let commitment_input = to_bytes![
            address_public_key.public_key, // 256 bits = 32 bytes
            is_dummy,                      // 1 bit = 1 byte
            payload,                       // 256 bits = 32 bytes
            birth_predicate_repr,          // 256 bits = 32 bytes
            death_predicate_repr,          // 256 bits = 32 bytes
            sn_nonce                       // 256 bits = 32 bytes
        ]?;

        let commitment = Components::RecC::commit(
            &parameters.rec_comm_pp,
            &commitment_input,
            &commitment_randomness,
        )?;

        let record = DPCRecord {
            address_public_key: address_public_key.clone(),
            is_dummy,
            payload: *payload,
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

    pub fn create_address_helper<R: Rng>(
        parameters: &CommAndCRHPublicParameters<Components>,
        metadata: &[u8; 32],
        rng: &mut R,
    ) -> Result<AddressPair<Components>, Error> {
        // Sample PRF secret key.
        let sk_bytes: [u8; 32] = rng.gen();
        let sk_prf: <Components::P as PRF>::Seed = FromBytes::read(sk_bytes.as_ref())?;

        // Sample randomness rpk for the commitment scheme.
        let r_pk = <Components::AddrC as CommitmentScheme>::Randomness::rand(rng);

        // Construct the address public key.
        let commit_input = to_bytes![sk_prf, metadata]?;
        let public_key = Components::AddrC::commit(&parameters.addr_comm_pp, &commit_input, &r_pk)?;
        let public_key = AddressPublicKey { public_key };

        // Construct the address secret key.
        let secret_key = AddressSecretKey {
            sk_prf,
            metadata: *metadata,
            r_pk,
        };

        Ok(AddressPair {
            public_key,
            secret_key,
        })
    }

    pub(crate) fn execute_helper<'a, L, R: Rng>(
        parameters: &'a CommAndCRHPublicParameters<Components>,

        old_records: &'a [<Self as DPCScheme<L>>::Record],
        old_address_secret_keys: &'a [AddressSecretKey<Components>],

        new_address_public_keys: &[AddressPublicKey<Components>],
        new_is_dummy_flags: &[bool],
        new_payloads: &[<Self as DPCScheme<L>>::Payload],
        new_birth_predicates: &[<Self as DPCScheme<L>>::Predicate],
        new_death_predicates: &[<Self as DPCScheme<L>>::Predicate],

        memo: &[u8; 32],
        auxiliary: &[u8; 32],

        ledger: &L,
        rng: &mut R,
    ) -> Result<ExecuteContext<'a, Components>, Error>
    where
        L: Ledger<
            Parameters = Components::MerkleParameters,
            Commitment = <Components::RecC as CommitmentScheme>::Output,
            SerialNumber = <Components::P as PRF>::Output,
        >,
    {
        assert_eq!(Components::NUM_INPUT_RECORDS, old_records.len());
        assert_eq!(Components::NUM_INPUT_RECORDS, old_address_secret_keys.len());

        assert_eq!(
            Components::NUM_OUTPUT_RECORDS,
            new_address_public_keys.len()
        );
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_is_dummy_flags.len());
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_payloads.len());
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_birth_predicates.len());
        assert_eq!(Components::NUM_OUTPUT_RECORDS, new_death_predicates.len());

        let mut old_witnesses = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        let mut old_serial_numbers = Vec::with_capacity(Components::NUM_INPUT_RECORDS);
        let mut joint_serial_numbers = Vec::new();
        let mut old_death_pred_hashes = Vec::new();

        // Compute the ledger membership witness and serial number from the old records.
        for (i, record) in old_records.iter().enumerate() {
            let input_record_time = start_timer!(|| format!("Process input record {}", i));

            if record.is_dummy() {
                old_witnesses.push(MerklePath::default());
            } else {
                let comm = &record.commitment();
                let witness = ledger.prove_cm(comm)?;
                old_witnesses.push(witness);
            }

            let sn = Self::generate_sn(record, &old_address_secret_keys[i])?;
            joint_serial_numbers.extend_from_slice(&to_bytes![sn]?);
            old_serial_numbers.push(sn);
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
            let sn_nonce = Components::SnNonceH::evaluate(&parameters.sn_nonce_crh_pp, &crh_input)?;

            end_timer!(sn_nonce_time);

            let record = Self::generate_record(
                parameters,
                &sn_nonce,
                &new_address_public_keys[j],
                new_is_dummy_flags[j],
                &new_payloads[j],
                &new_birth_predicates[j],
                &new_death_predicates[j],
                rng,
            )?;

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
                record.address_public_key(),
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
                record.address_public_key(),
                record.is_dummy(),
                record.payload(),
                record.birth_predicate_repr(),
                record.death_predicate_repr()
            ]?;
            predicate_input.extend_from_slice(&bytes);
        }
        predicate_input.extend_from_slice(memo);
        predicate_input.extend_from_slice(auxiliary);

        let local_data_rand =
            <Components::LocalDataComm as CommitmentScheme>::Randomness::rand(rng);
        let local_data_comm = Components::LocalDataComm::commit(
            &parameters.local_data_comm_pp,
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
                <Components::PredVkComm as CommitmentScheme>::Randomness::rand(rng);
            let predicate_comm = Components::PredVkComm::commit(
                &parameters.pred_vk_comm_pp,
                &input,
                &predicate_rand,
            )?;
            (predicate_comm, predicate_rand)
        };
        end_timer!(pred_hash_comm_timer);

        let ledger_digest = ledger.digest().expect("could not get digest");

        let context = ExecuteContext {
            comm_and_crh_pp: parameters,
            ledger_digest,

            old_records,
            old_witnesses,
            old_address_secret_keys,
            old_serial_numbers,

            new_records,
            new_sn_nonce_randomness,
            new_commitments,

            predicate_comm,
            predicate_rand,
            local_data_comm,
            local_data_rand,
        };
        Ok(context)
    }
}

impl<Components: PlainDPCComponents, L: Ledger> DPCScheme<L> for DPC<Components>
where
    L: Ledger<
        Parameters = Components::MerkleParameters,
        Commitment = <Components::RecC as CommitmentScheme>::Output,
        SerialNumber = <Components::P as PRF>::Output,
    >,
{
    type AddressKeyPair = AddressPair<Components>;
    type Auxiliary = [u8; 32];
    type Metadata = [u8; 32];
    type Payload = <Self::Record as Record>::Payload;
    type Parameters = PublicParameters<Components>;
    type Predicate = DPCPredicate<Components>;
    type PrivatePredInput = PrivatePredInput<Components>;
    type Record = DPCRecord<Components>;
    type Transaction = DPCTransaction<Components>;
    type LocalData = LocalData<Components>;

    fn setup<R: Rng>(ledger_pp: &MerkleTreeParams<Components::MerkleParameters>, rng: &mut R) -> Result<Self::Parameters, Error> {
        let setup_time = start_timer!(|| "PlainDPC::Setup");
        let comm_and_crh_pp = Self::generate_comm_and_crh_parameters(rng)?;

        let pred_nizk_setup_time = start_timer!(|| "Dummy Predicate NIZK Setup");
        let pred_nizk_pp = Self::generate_pred_nizk_parameters(&comm_and_crh_pp, rng)?;
        end_timer!(pred_nizk_setup_time);

        let private_pred_input = PrivatePredInput {
            vk:    pred_nizk_pp.vk.clone(),
            proof: pred_nizk_pp.proof.clone(),
        };

        let nizk_setup_time = start_timer!(|| "Execute Tx Core Checks NIZK Setup");
        let core_nizk_pp = Components::MainNIZK::setup(
            CoreChecksCircuit::blank(&comm_and_crh_pp, ledger_pp),
            rng,
        )?;
        end_timer!(nizk_setup_time);

        let nizk_setup_time = start_timer!(|| "Execute Tx Proof Checks NIZK Setup");
        let proof_check_nizk_pp = Components::ProofCheckNIZK::setup(
            ProofCheckCircuit::blank(&comm_and_crh_pp, &private_pred_input),
            rng,
        )?;
        end_timer!(nizk_setup_time);
        end_timer!(setup_time);
        Ok(PublicParameters {
            comm_and_crh_pp,
            pred_nizk_pp,
            core_nizk_pp,
            proof_check_nizk_pp,
        })
    }

    fn create_address<R: Rng>(
        parameters: &Self::Parameters,
        metadata: &Self::Metadata,
        rng: &mut R,
    ) -> Result<Self::AddressKeyPair, Error> {
        let create_addr_time = start_timer!(|| "PlainDPC::CreateAddr");
        let result = Self::create_address_helper(&parameters.comm_and_crh_pp, metadata, rng)?;
        end_timer!(create_addr_time);
        Ok(result)
    }

    fn execute<R: Rng>(
        parameters: &Self::Parameters,
        old_records: &[Self::Record],
        old_address_secret_keys: &[<Self::AddressKeyPair as AddressKeyPair>::AddressSecretKey],
        mut old_death_pred_proof_generator: impl FnMut(&Self::LocalData) -> Vec<Self::PrivatePredInput>,

        new_address_public_keys: &[<Self::AddressKeyPair as AddressKeyPair>::AddressPublicKey],
        new_is_dummy_flags: &[bool],
        new_payloads: &[Self::Payload],
        new_birth_predicates: &[Self::Predicate],
        new_death_predicates: &[Self::Predicate],
        mut new_birth_pred_proof_generator: impl FnMut(&Self::LocalData) -> Vec<Self::PrivatePredInput>,

        auxiliary: &Self::Auxiliary,
        memorandum: &<Self::Transaction as Transaction>::Memorandum,
        ledger: &L,
        rng: &mut R,
    ) -> Result<(Vec<Self::Record>, Self::Transaction), Error> {
        let exec_time = start_timer!(|| "PlainDPC::Exec");
        let context = Self::execute_helper(
            &parameters.comm_and_crh_pp,
            old_records,
            old_address_secret_keys,
            new_address_public_keys,
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
        let old_death_pred_vk_and_proofs = old_death_pred_proof_generator(&local_data);
        let new_birth_pred_vk_and_proofs = new_birth_pred_proof_generator(&local_data);

        let ExecuteContext {
            comm_and_crh_pp: _comm_and_crh_pp,
            ledger_digest,

            old_records,
            old_witnesses,
            old_address_secret_keys,
            old_serial_numbers,

            new_records,
            new_sn_nonce_randomness,
            new_commitments,

            predicate_comm,
            predicate_rand,

            local_data_comm,
            local_data_rand,
        } = context;
        let core_proof = {
            let circuit = CoreChecksCircuit::new(
                &parameters.comm_and_crh_pp,
                ledger.parameters(),
                &ledger_digest,
                old_records,
                &old_witnesses,
                old_address_secret_keys,
                &old_serial_numbers,
                &new_records,
                &new_sn_nonce_randomness,
                &new_commitments,
                &predicate_comm,
                &predicate_rand,
                &local_data_comm,
                &local_data_rand,
                memorandum,
                auxiliary,
            );

            Components::MainNIZK::prove(&parameters.core_nizk_pp.0, circuit, rng)?
        };

        let proof_checks_proof = {
            let circuit = ProofCheckCircuit::new(
                &parameters.comm_and_crh_pp,
                old_death_pred_vk_and_proofs.as_slice(),
                new_birth_pred_vk_and_proofs.as_slice(),
                &predicate_comm,
                &predicate_rand,
                &local_data_comm,
            );

            Components::ProofCheckNIZK::prove(&parameters.proof_check_nizk_pp.0, circuit, rng)?
        };

        let transaction = Self::Transaction::new(
            old_serial_numbers,
            new_commitments,
            memorandum.clone(),
            ledger_digest,
            core_proof,
            proof_checks_proof,
            predicate_comm,
            local_data_comm,
        );

        end_timer!(exec_time);
        Ok((new_records, transaction))
    }

    fn verify(
        parameters: &Self::Parameters,
        transaction: &Self::Transaction,
        ledger: &L,
    ) -> Result<bool, Error> {
        let verify_time = start_timer!(|| "PlainDPC::Verify");
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

        let input = CoreChecksVerifierInput {
            comm_and_crh_pp:    parameters.comm_and_crh_pp.clone(),
            ledger_pp:          ledger.parameters().clone(),
            ledger_digest:      transaction.stuff.digest.clone(),
            old_serial_numbers: transaction.old_serial_numbers().to_vec(),
            new_commitments:    transaction.new_commitments().to_vec(),
            memo:               transaction.memorandum().clone(),
            predicate_comm:     transaction.stuff.predicate_comm.clone(),
            local_data_comm:    transaction.stuff.local_data_comm.clone(),
        };
        if !Components::MainNIZK::verify(
            &parameters.core_nizk_pp.1,
            &input,
            &transaction.stuff.core_proof,
        )? {
            eprintln!("Core NIZK didn't verify.");
            return Ok(false);
        };

        let input = ProofCheckVerifierInput {
            comm_and_crh_pp: parameters.comm_and_crh_pp.clone(),
            predicate_comm:  transaction.stuff.predicate_comm.clone(),
            local_data_comm: transaction.stuff.local_data_comm.clone(),
        };

        if !Components::ProofCheckNIZK::verify(
            &parameters.proof_check_nizk_pp.1,
            &input,
            &transaction.stuff.predicate_proof,
        )? {
            eprintln!("Predicate check NIZK didn't verify.");
            return Ok(false);
        }
        end_timer!(verify_time);
        Ok(true)
    }
}
