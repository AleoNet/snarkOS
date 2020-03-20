use crate::{
    dpc::{
        address::AddressSecretKey,
        plain_dpc::{
            parameters::CommAndCRHPublicParameters,
            predicate::PrivatePredInput,
            record::DPCRecord,
            PlainDPCComponents,
        },
        Record,
    },
    ledger::MerkleTreeParams,
};
use snarkos_algorithms::merkle_tree::{MerklePath, MerkleTreeDigest};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_gadgets::algorithms::merkle_tree::merkle_path::MerklePathGadget;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH, PRF},
    curves::to_field_vec::ToConstraintField,
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget, PRFGadget, SNARKVerifierGadget},
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, boolean::Boolean, eq::EqGadget, uint8::UInt8, ToBytesGadget},
    },
};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

pub fn execute_core_checks_gadget<C: PlainDPCComponents, CS: ConstraintSystem<C::InnerField>>(
    cs: &mut CS,
    // Parameters
    comm_crh_parameters: &CommAndCRHPublicParameters<C>,
    ledger_parameters: &MerkleTreeParams<C::MerkleParameters>,

    // Digest
    ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,

    // Old record stuff
    old_records: &[DPCRecord<C>],
    old_witnesses: &[MerklePath<C::MerkleParameters>],
    old_address_secret_keys: &[AddressSecretKey<C>],
    old_serial_numbers: &[<C::P as PRF>::Output],

    // New record stuff
    new_records: &[DPCRecord<C>],
    new_sn_nonce_randomness: &[[u8; 32]],
    new_commitments: &[<C::RecordCommitment as CommitmentScheme>::Output],

    // Rest
    predicate_comm: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    predicate_rand: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,
    local_data_comm: &<C::LocalDataCommitment as CommitmentScheme>::Output,
    local_data_rand: &<C::LocalDataCommitment as CommitmentScheme>::Randomness,
    memo: &[u8; 32],
    auxiliary: &[u8; 32],
) -> Result<(), SynthesisError> {
    execute_core_checks_gadget_helper::<
        C,
        CS,
        C::AddressCommitment,
        C::RecordCommitment,
        C::SerialNumberNonce,
        C::P,
        C::AddressCommitmentGadget,
        C::RecordCommitmentGadget,
        C::SerialNumberNonceGadget,
        C::PGadget,
    >(
        cs,
        //
        comm_crh_parameters,
        ledger_parameters,
        //
        ledger_digest,
        //
        old_records,
        old_witnesses,
        old_address_secret_keys,
        old_serial_numbers,
        //
        new_records,
        new_sn_nonce_randomness,
        new_commitments,
        //
        predicate_comm,
        predicate_rand,
        local_data_comm,
        local_data_rand,
        memo,
        auxiliary,
    )
}

fn execute_core_checks_gadget_helper<
    C,
    CS: ConstraintSystem<C::InnerField>,
    AddrC,
    RecC,
    SnNonceH,
    P,
    AddrCGadget,
    RecCGadget,
    SnNonceHGadget,
    PGadget,
>(
    cs: &mut CS,

    //
    comm_crh_parameters: &CommAndCRHPublicParameters<C>,
    ledger_parameters: &MerkleTreeParams<C::MerkleParameters>,

    //
    ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,

    //
    old_records: &[DPCRecord<C>],
    old_witnesses: &[MerklePath<C::MerkleParameters>],
    old_address_secret_keys: &[AddressSecretKey<C>],
    old_serial_numbers: &[P::Output],

    //
    new_records: &[DPCRecord<C>],
    new_sn_nonce_randomness: &[[u8; 32]],
    new_commitments: &[RecC::Output],

    //
    predicate_comm: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    predicate_rand: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,
    local_data_comm: &<C::LocalDataCommitment as CommitmentScheme>::Output,
    local_data_rand: &<C::LocalDataCommitment as CommitmentScheme>::Randomness,
    memo: &[u8; 32],
    auxiliary: &[u8; 32],
) -> Result<(), SynthesisError>
where
    C: PlainDPCComponents<
        AddressCommitment = AddrC,
        RecordCommitment = RecC,
        SerialNumberNonce = SnNonceH,
        P = P,
        AddressCommitmentGadget = AddrCGadget,
        SerialNumberNonceGadget = SnNonceHGadget,
        RecordCommitmentGadget = RecCGadget,
        PGadget = PGadget,
    >,
    AddrC: CommitmentScheme,
    RecC: CommitmentScheme,
    SnNonceH: CRH,
    P: PRF,
    RecC::Output: Eq,
    AddrCGadget: CommitmentGadget<AddrC, C::InnerField>,
    RecCGadget: CommitmentGadget<RecC, C::InnerField>,
    SnNonceHGadget: CRHGadget<SnNonceH, C::InnerField>,
    PGadget: PRFGadget<P, C::InnerField>,
{
    let mut old_sns = Vec::with_capacity(old_records.len());
    let mut old_rec_comms = Vec::with_capacity(old_records.len());
    let mut old_apks = Vec::with_capacity(old_records.len());
    let mut old_dummy_flags = Vec::with_capacity(old_records.len());
    let mut old_payloads = Vec::with_capacity(old_records.len());
    let mut old_birth_pred_hashes = Vec::with_capacity(old_records.len());
    let mut old_death_pred_hashes = Vec::with_capacity(old_records.len());

    let mut new_rec_comms = Vec::with_capacity(new_records.len());
    let mut new_apks = Vec::with_capacity(new_records.len());
    let mut new_dummy_flags = Vec::with_capacity(new_records.len());
    let mut new_payloads = Vec::with_capacity(new_records.len());
    let mut new_death_pred_hashes = Vec::with_capacity(new_records.len());
    let mut new_birth_pred_hashes = Vec::with_capacity(new_records.len());

    // Order for allocation of input:
    // 1. addr_comm_pp.
    // 2. rec_comm_pp.
    // 3. crh_pp.
    // 4. ledger_parameters.
    // 5. ledger_digest.
    // 6. for i in 0..NUM_INPUT_RECORDS: old_serial_numbers[i].
    // 7. for j in 0..NUM_OUTPUT_RECORDS: new_commitments[i].
    let (addr_comm_pp, rec_comm_pp, pred_vk_comm_pp, local_data_comm_pp, sn_nonce_crh_pp, ledger_pp) = {
        let cs = &mut cs.ns(|| "Declare Comm and CRH parameters");
        let addr_comm_pp =
            AddrCGadget::ParametersGadget::alloc_input(&mut cs.ns(|| "Declare Addr Comm parameters"), || {
                Ok(comm_crh_parameters.addr_comm_pp.parameters())
            })?;

        let rec_comm_pp =
            RecCGadget::ParametersGadget::alloc_input(&mut cs.ns(|| "Declare Rec Comm parameters"), || {
                Ok(comm_crh_parameters.rec_comm_pp.parameters())
            })?;

        let local_data_comm_pp =
            <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::ParametersGadget::alloc_input(
                &mut cs.ns(|| "Declare Local Data Comm parameters"),
                || Ok(comm_crh_parameters.local_data_comm_pp.parameters()),
            )?;

        let pred_vk_comm_pp =
            <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<_, C::InnerField>>::ParametersGadget::alloc_input(
                &mut cs.ns(|| "Declare Pred Vk COMM parameters"),
                || Ok(comm_crh_parameters.pred_vk_comm_pp.parameters()),
            )?;

        let sn_nonce_crh_pp =
            SnNonceHGadget::ParametersGadget::alloc_input(&mut cs.ns(|| "Declare SN Nonce CRH parameters"), || {
                Ok(comm_crh_parameters.sn_nonce_crh_pp.parameters())
            })?;

        let ledger_pp = <C::MerkleHashGadget as CRHGadget<_, _>>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare Ledger Parameters"),
            || Ok(ledger_parameters.parameters()),
        )?;
        (
            addr_comm_pp,
            rec_comm_pp,
            pred_vk_comm_pp,
            local_data_comm_pp,
            sn_nonce_crh_pp,
            ledger_pp,
        )
    };

    let digest_gadget = <C::MerkleHashGadget as CRHGadget<_, _>>::OutputGadget::alloc_input(
        &mut cs.ns(|| "Declare ledger digest"),
        || Ok(ledger_digest),
    )?;

    for (i, (((record, witness), secret_key), given_serial_number)) in old_records
        .iter()
        .zip(old_witnesses)
        .zip(old_address_secret_keys)
        .zip(old_serial_numbers)
        .enumerate()
    {
        let cs = &mut cs.ns(|| format!("Process input record {}", i));
        // Declare record contents
        let (
            given_apk,
            given_commitment,
            given_is_dummy,
            given_payload,
            given_birth_pred_hash,
            given_death_pred_hash,
            given_comm_rand,
            sn_nonce,
        ) = {
            let declare_cs = &mut cs.ns(|| "Declare input record");
            // No need to check that commitments, public keys and hashes are in
            // prime order subgroup because the commitment and CRH parameters
            // are trusted, and so when we recompute these, the newly computed
            // values will always be in correct subgroup. If the input cm, pk
            // or hash is incorrect, then it will not match the computed equivalent.
            let given_apk = AddrCGadget::OutputGadget::alloc(&mut declare_cs.ns(|| "Addr PubKey"), || {
                Ok(&record.address_public_key().public_key)
            })?;
            old_apks.push(given_apk.clone());

            let given_commitment = RecCGadget::OutputGadget::alloc(&mut declare_cs.ns(|| "Commitment"), || {
                Ok(record.commitment().clone())
            })?;
            old_rec_comms.push(given_commitment.clone());

            let given_is_dummy = Boolean::alloc(&mut declare_cs.ns(|| "is_dummy"), || Ok(record.is_dummy()))?;
            old_dummy_flags.push(given_is_dummy.clone());

            let given_payload = UInt8::alloc_vec(&mut declare_cs.ns(|| "Payload"), record.payload())?;
            old_payloads.push(given_payload.clone());

            let given_birth_pred_hash =
                UInt8::alloc_vec(&mut declare_cs.ns(|| "Birth predicate"), &record.birth_predicate_repr())?;
            old_birth_pred_hashes.push(given_birth_pred_hash.clone());

            let given_death_pred_hash =
                UInt8::alloc_vec(&mut declare_cs.ns(|| "Death predicate"), &record.death_predicate_repr())?;
            old_death_pred_hashes.push(given_death_pred_hash.clone());

            let given_comm_rand =
                RecCGadget::RandomnessGadget::alloc(&mut declare_cs.ns(|| "Commitment randomness"), || {
                    Ok(record.commitment_randomness())
                })?;

            let sn_nonce = SnNonceHGadget::OutputGadget::alloc(&mut declare_cs.ns(|| "Sn nonce"), || {
                Ok(record.serial_number_nonce())
            })?;
            (
                given_apk,
                given_commitment,
                given_is_dummy,
                given_payload,
                given_birth_pred_hash,
                given_death_pred_hash,
                given_comm_rand,
                sn_nonce,
            )
        };

        // ********************************************************************
        // Check that the commitment appears on the ledger,
        // i.e., the membership witness is valid with respect to the
        // transaction set digest.
        // ********************************************************************
        {
            let witness_cs = &mut cs.ns(|| "Check membership witness");

            let witness_gadget =
                MerklePathGadget::<_, C::MerkleHashGadget, _>::alloc(&mut witness_cs.ns(|| "Declare witness"), || {
                    Ok(witness)
                })?;

            witness_gadget.conditionally_check_membership(
                &mut witness_cs.ns(|| "Perform check"),
                &ledger_pp,
                &digest_gadget,
                &given_commitment,
                &given_is_dummy.not(),
            )?;
        }
        // ********************************************************************

        // ********************************************************************
        // Check that the address public key and secret key form a valid key
        // pair.
        // ********************************************************************

        let sk_prf = {
            // Declare variables for addr_sk contents.
            let address_cs = &mut cs.ns(|| "Check address keypair");
            let sk_prf = PGadget::new_seed(&mut address_cs.ns(|| "Declare sk_prf"), &secret_key.sk_prf);
            let metadata = UInt8::alloc_vec(&mut address_cs.ns(|| "Declare metadata"), &secret_key.metadata)?;
            let r_pk =
                AddrCGadget::RandomnessGadget::alloc(&mut address_cs.ns(|| "Declare r_pk"), || Ok(&secret_key.r_pk))?;

            let mut apk_input = sk_prf.clone();
            apk_input.extend_from_slice(&metadata);

            let candidate_apk = AddrCGadget::check_commitment_gadget(
                &mut address_cs.ns(|| "Compute Addr PubKey"),
                &addr_comm_pp,
                &apk_input,
                &r_pk,
            )?;

            candidate_apk.enforce_equal(
                &mut address_cs.ns(|| "Check that declared and computed pks are equal"),
                &given_apk,
            )?;
            sk_prf
        };
        // ********************************************************************

        // ********************************************************************
        // Check that the serial number is derived correctly.
        // ********************************************************************
        let sn_nonce_bytes = {
            let sn_cs = &mut cs.ns(|| "Check that sn is derived correctly");

            let sn_nonce_bytes = sn_nonce.to_bytes(&mut sn_cs.ns(|| "Convert nonce to bytes"))?;

            let prf_seed = sk_prf;
            let candidate_serial_number = PGadget::check_evaluation_gadget(
                &mut sn_cs.ns(|| "Compute serial number"),
                &prf_seed,
                &sn_nonce_bytes,
            )?;

            let given_sn = PGadget::OutputGadget::alloc_input(&mut sn_cs.ns(|| "Declare given serial number"), || {
                Ok(given_serial_number)
            })?;

            candidate_serial_number.enforce_equal(
                &mut sn_cs.ns(|| "Check that given and computed serial numbers are equal"),
                &given_sn,
            )?;
            old_sns.push(candidate_serial_number);
            sn_nonce_bytes
        };
        // ********************************************************************

        // Check that the record is well-formed.
        {
            let comm_cs = &mut cs.ns(|| "Check that record is well-formed");
            let apk_bytes = given_apk.to_bytes(&mut comm_cs.ns(|| "Convert apk to bytes"))?;
            let is_dummy_bytes = given_is_dummy.to_bytes(&mut comm_cs.ns(|| "Convert is_dummy to bytes"))?;

            let mut comm_input = Vec::new();
            comm_input.extend_from_slice(&apk_bytes);
            comm_input.extend_from_slice(&is_dummy_bytes);
            comm_input.extend_from_slice(&given_payload);
            comm_input.extend_from_slice(&given_birth_pred_hash);
            comm_input.extend_from_slice(&given_death_pred_hash);
            comm_input.extend_from_slice(&sn_nonce_bytes);
            let candidate_commitment = RecCGadget::check_commitment_gadget(
                &mut comm_cs.ns(|| "Compute commitment"),
                &rec_comm_pp,
                &comm_input,
                &given_comm_rand,
            )?;
            candidate_commitment.enforce_equal(
                &mut comm_cs.ns(|| "Check that declared and computed commitments are equal"),
                &given_commitment,
            )?;
        }
    }

    let sn_nonce_input = {
        let cs = &mut cs.ns(|| "Convert input serial numbers to bytes");
        let mut sn_nonce_input = Vec::new();
        for (i, old_sn) in old_sns.iter().enumerate() {
            let bytes = old_sn.to_bytes(&mut cs.ns(|| format!("Convert {}-th serial number to bytes", i)))?;
            sn_nonce_input.extend_from_slice(&bytes);
        }
        sn_nonce_input
    };

    for (j, ((record, sn_nonce_rand), commitment)) in new_records
        .iter()
        .zip(new_sn_nonce_randomness)
        .zip(new_commitments)
        .enumerate()
    {
        let cs = &mut cs.ns(|| format!("Process output record {}", j));
        let j = j as u8;

        let (
            given_apk,
            given_record_comm,
            given_comm,
            given_is_dummy,
            given_payload,
            given_birth_pred_hash,
            given_death_pred_hash,
            given_comm_rand,
            sn_nonce,
        ) = {
            let declare_cs = &mut cs.ns(|| "Declare output record");
            let given_apk = AddrCGadget::OutputGadget::alloc(&mut declare_cs.ns(|| "Addr PubKey"), || {
                Ok(&record.address_public_key().public_key)
            })?;
            new_apks.push(given_apk.clone());
            let given_record_comm = RecCGadget::OutputGadget::alloc(
                &mut declare_cs.ns(|| "Record Commitment"),
                || Ok(record.commitment()),
            )?;
            new_rec_comms.push(given_record_comm.clone());
            let given_comm =
                RecCGadget::OutputGadget::alloc_input(&mut declare_cs.ns(|| "Given Commitment"), || Ok(commitment))?;

            let given_is_dummy = Boolean::alloc(&mut declare_cs.ns(|| "is_dummy"), || Ok(record.is_dummy()))?;
            new_dummy_flags.push(given_is_dummy.clone());

            let given_payload = UInt8::alloc_vec(&mut declare_cs.ns(|| "Payload"), record.payload())?;
            new_payloads.push(given_payload.clone());

            let given_birth_pred_hash =
                UInt8::alloc_vec(&mut declare_cs.ns(|| "Birth predicate"), &record.birth_predicate_repr())?;
            new_birth_pred_hashes.push(given_birth_pred_hash.clone());
            let given_death_pred_hash =
                UInt8::alloc_vec(&mut declare_cs.ns(|| "Death predicate"), &record.death_predicate_repr())?;
            new_death_pred_hashes.push(given_death_pred_hash.clone());

            let given_comm_rand =
                RecCGadget::RandomnessGadget::alloc(&mut declare_cs.ns(|| "Commitment randomness"), || {
                    Ok(record.commitment_randomness())
                })?;

            let sn_nonce = SnNonceHGadget::OutputGadget::alloc(&mut declare_cs.ns(|| "Sn nonce"), || {
                Ok(record.serial_number_nonce())
            })?;

            (
                given_apk,
                given_record_comm,
                given_comm,
                given_is_dummy,
                given_payload,
                given_birth_pred_hash,
                given_death_pred_hash,
                given_comm_rand,
                sn_nonce,
            )
        };

        // *******************************************************************
        // Check that the serial number nonce is computed correctly.
        // *******************************************************************
        {
            let sn_cs = &mut cs.ns(|| "Check that serial number nonce is computed correctly");

            let cur_record_num = UInt8::constant(j);
            let mut cur_record_num_bytes_le = vec![cur_record_num];

            let sn_nonce_randomness =
                UInt8::alloc_vec(sn_cs.ns(|| "Allocate serial number nonce randomness"), sn_nonce_rand)?;
            cur_record_num_bytes_le.extend_from_slice(&sn_nonce_randomness);
            cur_record_num_bytes_le.extend_from_slice(&sn_nonce_input);

            let sn_nonce_input = cur_record_num_bytes_le;

            let candidate_sn_nonce = SnNonceHGadget::check_evaluation_gadget(
                &mut sn_cs.ns(|| "Compute serial number nonce"),
                &sn_nonce_crh_pp,
                &sn_nonce_input,
            )?;
            candidate_sn_nonce.enforce_equal(
                &mut sn_cs.ns(|| "Check that computed nonce matches provided nonce"),
                &sn_nonce,
            )?;
        }
        // *******************************************************************

        // *******************************************************************
        // Check that the record is well-formed.
        // *******************************************************************
        {
            let comm_cs = &mut cs.ns(|| "Check that record is well-formed");
            let apk_bytes = given_apk.to_bytes(&mut comm_cs.ns(|| "Convert Addr PubKey to bytes"))?;
            let is_dummy_bytes = given_is_dummy.to_bytes(&mut comm_cs.ns(|| "Convert is_dummy to bytes"))?;
            let sn_nonce_bytes = sn_nonce.to_bytes(&mut comm_cs.ns(|| "Convert sn nonce to bytes"))?;

            let mut comm_input = Vec::new();
            comm_input.extend_from_slice(&apk_bytes);
            comm_input.extend_from_slice(&is_dummy_bytes);
            comm_input.extend_from_slice(&given_payload);
            comm_input.extend_from_slice(&given_birth_pred_hash);
            comm_input.extend_from_slice(&given_death_pred_hash);
            comm_input.extend_from_slice(&sn_nonce_bytes);

            let candidate_commitment = RecCGadget::check_commitment_gadget(
                &mut comm_cs.ns(|| "Compute record commitment"),
                &rec_comm_pp,
                &comm_input,
                &given_comm_rand,
            )?;
            candidate_commitment.enforce_equal(
                &mut comm_cs.ns(|| "Check that computed commitment matches pub input"),
                &given_comm,
            )?;
            candidate_commitment.enforce_equal(
                &mut comm_cs.ns(|| "Check that computed commitment matches declared comm"),
                &given_record_comm,
            )?;
        }
    }
    // *******************************************************************
    // Check that predicate commitment is well formed.
    // *******************************************************************
    {
        let comm_cs = &mut cs.ns(|| "Check that predicate commitment is well-formed");

        let mut input = Vec::new();
        for i in 0..C::NUM_INPUT_RECORDS {
            input.extend_from_slice(&old_death_pred_hashes[i]);
        }

        for j in 0..C::NUM_OUTPUT_RECORDS {
            input.extend_from_slice(&new_birth_pred_hashes[j]);
        }

        let given_comm_rand = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<_, C::InnerField>>::RandomnessGadget::alloc(
            &mut comm_cs.ns(|| "Commitment randomness"),
            || Ok(predicate_rand),
        )?;

        let given_comm = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<_, C::InnerField>>::OutputGadget::alloc_input(
            &mut comm_cs.ns(|| "Commitment output"),
            || Ok(predicate_comm),
        )?;

        let candidate_commitment = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<
            _,
            C::InnerField,
        >>::check_commitment_gadget(
            &mut comm_cs.ns(|| "Compute commitment"),
            &pred_vk_comm_pp,
            &input,
            &given_comm_rand,
        )?;

        candidate_commitment.enforce_equal(
            &mut comm_cs.ns(|| "Check that declared and computed commitments are equal"),
            &given_comm,
        )?;
    }
    {
        let mut cs = cs.ns(|| "Check that local data commitment is valid.");

        let mut local_data_bytes = Vec::new();
        for i in 0..C::NUM_INPUT_RECORDS {
            let mut cs = cs.ns(|| format!("Construct local data with Input Record {}", i));
            local_data_bytes.extend_from_slice(&old_rec_comms[i].to_bytes(&mut cs.ns(|| "Record Comm"))?);
            local_data_bytes.extend_from_slice(&old_apks[i].to_bytes(&mut cs.ns(|| "Apk"))?);
            local_data_bytes.extend_from_slice(&old_dummy_flags[i].to_bytes(&mut cs.ns(|| "IsDummy"))?);
            local_data_bytes.extend_from_slice(&old_payloads[i]);
            local_data_bytes.extend_from_slice(&old_birth_pred_hashes[i]);
            local_data_bytes.extend_from_slice(&old_death_pred_hashes[i]);
            local_data_bytes.extend_from_slice(&old_sns[i].to_bytes(&mut cs.ns(|| "Sn"))?);
        }

        for j in 0..C::NUM_OUTPUT_RECORDS {
            let mut cs = cs.ns(|| format!("Construct local data with Output Record {}", j));
            local_data_bytes.extend_from_slice(&new_rec_comms[j].to_bytes(&mut cs.ns(|| "Record Comm"))?);
            local_data_bytes.extend_from_slice(&new_apks[j].to_bytes(&mut cs.ns(|| "Apk"))?);
            local_data_bytes.extend_from_slice(&new_dummy_flags[j].to_bytes(&mut cs.ns(|| "IsDummy"))?);
            local_data_bytes.extend_from_slice(&new_payloads[j]);
            local_data_bytes.extend_from_slice(&new_birth_pred_hashes[j]);
            local_data_bytes.extend_from_slice(&new_death_pred_hashes[j]);
        }
        let memo = UInt8::alloc_input_vec(cs.ns(|| "Allocate memorandum"), memo)?;
        local_data_bytes.extend_from_slice(&memo);

        let auxiliary = UInt8::alloc_vec(cs.ns(|| "Allocate auxiliary input"), auxiliary)?;
        local_data_bytes.extend_from_slice(&auxiliary);

        let local_data_comm_rand = <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::RandomnessGadget::alloc(
            cs.ns(|| "Allocate local data commitment randomness"),
            || Ok(local_data_rand),
        )?;

        let declared_local_data_comm =
            <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::OutputGadget::alloc_input(
                cs.ns(|| "Allocate local data commitment"),
                || Ok(local_data_comm),
            )?;

        let comm = C::LocalDataCommitmentGadget::check_commitment_gadget(
            cs.ns(|| "Commit to local data"),
            &local_data_comm_pp,
            &local_data_bytes,
            &local_data_comm_rand,
        )?;

        comm.enforce_equal(
            &mut cs.ns(|| "Check that local data commitment is valid"),
            &declared_local_data_comm,
        )?;
    }
    Ok(())
}

pub fn execute_proof_check_gadget<C: PlainDPCComponents, CS: ConstraintSystem<C::OuterField>>(
    cs: &mut CS,
    // Parameters
    comm_crh_parameters: &CommAndCRHPublicParameters<C>,

    // Old record death predicate verif. keys and proofs
    old_death_pred_vk_and_pf: &[PrivatePredInput<C>],

    // New record birth predicate verif. keys and proofs
    new_birth_pred_vk_and_pf: &[PrivatePredInput<C>],

    // Rest
    predicate_comm: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    predicate_rand: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,

    local_data_comm: &<C::LocalDataCommitment as CommitmentScheme>::Output,
) -> Result<(), SynthesisError>
where
    <C::LocalDataCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,
    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
{
    // Declare public parameters.
    let (pred_vk_comm_pp, pred_vk_crh_pp) = {
        let cs = &mut cs.ns(|| "Declare Comm and CRH parameters");

        let pred_vk_comm_pp =
            <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<_, C::OuterField>>::ParametersGadget::alloc_input(
                &mut cs.ns(|| "Declare Pred Vk COMM parameters"),
                || Ok(comm_crh_parameters.pred_vk_comm_pp.parameters()),
            )?;

        let pred_vk_crh_pp =
            <C::PredicateVerificationKeyHashGadget as CRHGadget<_, C::OuterField>>::ParametersGadget::alloc_input(
                &mut cs.ns(|| "Declare Pred Vk CRH parameters"),
                || Ok(comm_crh_parameters.pred_vk_crh_pp.parameters()),
            )?;

        (pred_vk_comm_pp, pred_vk_crh_pp)
    };

    // ************************************************************************
    // Construct predicate input
    // ************************************************************************

    // First we convert the input for the predicates into `CoreCheckF` field elements
    let local_data_comm_pp_fe =
        ToConstraintField::<C::InnerField>::to_field_elements(comm_crh_parameters.local_data_comm_pp.parameters())
            .map_err(|_| SynthesisError::AssignmentMissing)?;

    let local_data_comm_fe = ToConstraintField::<C::InnerField>::to_field_elements(local_data_comm)
        .map_err(|_| SynthesisError::AssignmentMissing)?;

    // Then we convert these field elements into bytes
    let pred_input = [
        to_bytes![local_data_comm_pp_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
        to_bytes![local_data_comm_fe].map_err(|_| SynthesisError::AssignmentMissing)?,
    ];

    let pred_input_bytes = [
        UInt8::alloc_input_vec(cs.ns(|| "Allocate local data pp "), &pred_input[0])?,
        UInt8::alloc_input_vec(cs.ns(|| "Allocate local data comm"), &pred_input[1])?,
    ];

    let pred_input_bits = [
        pred_input_bytes[0]
            .iter()
            .flat_map(|byte| byte.into_bits_le())
            .collect::<Vec<_>>(),
        pred_input_bytes[1]
            .iter()
            .flat_map(|byte| byte.into_bits_le())
            .collect::<Vec<_>>(),
    ];
    // ************************************************************************
    // ************************************************************************

    let mut old_death_pred_hashes = Vec::new();
    let mut new_birth_pred_hashes = Vec::new();
    for i in 0..C::NUM_INPUT_RECORDS {
        let cs = &mut cs.ns(|| format!("Check death predicate for input record {}", i));

        let death_pred_proof = <C::PredicateNIZKGadget as SNARKVerifierGadget<_, _>>::ProofGadget::alloc(
            &mut cs.ns(|| "Allocate proof"),
            || Ok(&old_death_pred_vk_and_pf[i].proof),
        )?;

        let death_pred_vk = <C::PredicateNIZKGadget as SNARKVerifierGadget<_, _>>::VerificationKeyGadget::alloc(
            &mut cs.ns(|| "Allocate verification key"),
            || Ok(&old_death_pred_vk_and_pf[i].vk),
        )?;

        let death_pred_vk_bytes = death_pred_vk.to_bytes(&mut cs.ns(|| "Convert death pred vk to bytes"))?;

        let claimed_death_pred_hash = C::PredicateVerificationKeyHashGadget::check_evaluation_gadget(
            &mut cs.ns(|| "Compute death predicate vk hash"),
            &pred_vk_crh_pp,
            &death_pred_vk_bytes,
        )?;

        let claimed_death_pred_hash_bytes =
            claimed_death_pred_hash.to_bytes(&mut cs.ns(|| "Convert death_pred vk hash to bytes"))?;

        old_death_pred_hashes.push(claimed_death_pred_hash_bytes);

        let position = UInt8::constant(i as u8).into_bits_le();
        C::PredicateNIZKGadget::check_verify(
            &mut cs.ns(|| "Check that proof is satisfied"),
            &death_pred_vk,
            ([position].iter())
                .chain(pred_input_bits.iter())
                .filter(|inp| !inp.is_empty()),
            &death_pred_proof,
        )?;
    }

    for j in 0..C::NUM_OUTPUT_RECORDS {
        let cs = &mut cs.ns(|| format!("Check birth predicate for output record {}", j));

        let birth_pred_proof = <C::PredicateNIZKGadget as SNARKVerifierGadget<_, _>>::ProofGadget::alloc(
            &mut cs.ns(|| "Allocate proof"),
            || Ok(&new_birth_pred_vk_and_pf[j].proof),
        )?;

        let birth_pred_vk = <C::PredicateNIZKGadget as SNARKVerifierGadget<_, _>>::VerificationKeyGadget::alloc(
            &mut cs.ns(|| "Allocate verification key"),
            || Ok(&new_birth_pred_vk_and_pf[j].vk),
        )?;

        let birth_pred_vk_bytes = birth_pred_vk.to_bytes(&mut cs.ns(|| "Convert birth pred vk to bytes"))?;

        let claimed_birth_pred_hash = C::PredicateVerificationKeyHashGadget::check_evaluation_gadget(
            &mut cs.ns(|| "Compute birth predicate vk hash"),
            &pred_vk_crh_pp,
            &birth_pred_vk_bytes,
        )?;

        let claimed_birth_pred_hash_bytes =
            claimed_birth_pred_hash.to_bytes(&mut cs.ns(|| "Convert birth_pred vk hash to bytes"))?;

        new_birth_pred_hashes.push(claimed_birth_pred_hash_bytes);

        let position = UInt8::constant(j as u8).into_bits_le();
        C::PredicateNIZKGadget::check_verify(
            &mut cs.ns(|| "Check that proof is satisfied"),
            &birth_pred_vk,
            ([position].iter())
                .chain(pred_input_bits.iter())
                .filter(|inp| !inp.is_empty()),
            &birth_pred_proof,
        )?;
    }
    {
        let comm_cs = &mut cs.ns(|| "Check that predicate commitment is well-formed");

        let mut input = Vec::new();
        for i in 0..C::NUM_INPUT_RECORDS {
            input.extend_from_slice(&old_death_pred_hashes[i]);
        }

        for j in 0..C::NUM_OUTPUT_RECORDS {
            input.extend_from_slice(&new_birth_pred_hashes[j]);
        }

        let given_comm_rand = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<_, C::OuterField>>::RandomnessGadget::alloc(
            &mut comm_cs.ns(|| "Commitment randomness"),
            || Ok(predicate_rand),
        )?;

        let given_comm = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<_, C::OuterField>>::OutputGadget::alloc_input(
            &mut comm_cs.ns(|| "Commitment output"),
            || Ok(predicate_comm),
        )?;

        let candidate_commitment = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<
            _,
            C::OuterField,
        >>::check_commitment_gadget(
            &mut comm_cs.ns(|| "Compute commitment"),
            &pred_vk_comm_pp,
            &input,
            &given_comm_rand,
        )?;

        candidate_commitment.enforce_equal(
            &mut comm_cs.ns(|| "Check that declared and computed commitments are equal"),
            &given_comm,
        )?;
    }
    Ok(())
}
