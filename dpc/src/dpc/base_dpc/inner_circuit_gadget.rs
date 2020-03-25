use crate::{
    dpc::{
        address::AddressSecretKey,
        base_dpc::{
            binding_signature::BindingSignature,
            parameters::CircuitParameters,
            record::DPCRecord,
            BaseDPCComponents,
        },
        Record,
    },
    ledger::MerkleTreeParameters,
};
use snarkos_algorithms::merkle_tree::{MerklePath, MerkleTreeDigest};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_gadgets::algorithms::merkle_tree::merkle_path::MerklePathGadget;
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, CRH, PRF},
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget, PRFGadget, SignaturePublicKeyRandomizationGadget},
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, boolean::Boolean, eq::EqGadget, uint8::UInt8, ToBytesGadget},
    },
};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

pub fn execute_inner_proof_gadget<C: BaseDPCComponents, CS: ConstraintSystem<C::InnerField>>(
    cs: &mut CS,
    // Parameters
    comm_crh_sig_parameters: &CircuitParameters<C>,
    ledger_parameters: &MerkleTreeParameters<C::MerkleParameters>,

    // Digest
    ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,

    // Old record stuff
    old_records: &[DPCRecord<C>],
    old_witnesses: &[MerklePath<C::MerkleParameters>],
    old_address_secret_keys: &[AddressSecretKey<C>],
    old_serial_numbers: &[<C::Signature as SignatureScheme>::PublicKey],

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
    input_value_commitments: &[[u8; 32]],
    output_value_commitments: &[[u8; 32]],
    value_balance: u64,
    binding_signature: &BindingSignature,
) -> Result<(), SynthesisError> {
    base_dpc_execute_gadget_helper::<
        C,
        CS,
        C::AddressCommitment,
        C::RecordCommitment,
        C::LocalDataCommitment,
        C::SerialNumberNonce,
        C::Signature,
        C::PRF,
        C::AddressCommitmentGadget,
        C::RecordCommitmentGadget,
        C::LocalDataCommitmentGadget,
        C::SerialNumberNonceGadget,
        C::SignatureGadget,
        C::PRFGadget,
    >(
        cs,
        //
        comm_crh_sig_parameters,
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
        input_value_commitments,
        output_value_commitments,
        value_balance,
        binding_signature,
    )
}

fn base_dpc_execute_gadget_helper<
    C,
    CS: ConstraintSystem<C::InnerField>,
    AddrC,
    RecC,
    LocalDataC,
    SnNonceH,
    SignatureS,
    P,
    AddrCGadget,
    RecCGadget,
    LocalDataCGadget,
    SnNonceHGadget,
    SignatureSGadget,
    PGadget,
>(
    cs: &mut CS,

    //
    comm_crh_sig_parameters: &CircuitParameters<C>,
    ledger_parameters: &MerkleTreeParameters<C::MerkleParameters>,

    //
    ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,

    //
    old_records: &[DPCRecord<C>],
    old_witnesses: &[MerklePath<C::MerkleParameters>],
    old_address_secret_keys: &[AddressSecretKey<C>],
    old_serial_numbers: &[SignatureS::PublicKey],

    //
    new_records: &[DPCRecord<C>],
    new_sn_nonce_randomness: &[[u8; 32]],
    new_commitments: &[RecC::Output],

    //
    predicate_comm: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    predicate_rand: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,
    local_data_comm: &LocalDataC::Output,
    local_data_rand: &LocalDataC::Randomness,
    memo: &[u8; 32],
    auxiliary: &[u8; 32],
    input_value_commitments: &[[u8; 32]],
    output_value_commitments: &[[u8; 32]],
    value_balance: u64,
    binding_signature: &BindingSignature,
) -> Result<(), SynthesisError>
where
    C: BaseDPCComponents<
        AddressCommitment = AddrC,
        RecordCommitment = RecC,
        LocalDataCommitment = LocalDataC,
        SerialNumberNonce = SnNonceH,
        Signature = SignatureS,
        PRF = P,
        AddressCommitmentGadget = AddrCGadget,
        RecordCommitmentGadget = RecCGadget,
        LocalDataCommitmentGadget = LocalDataCGadget,
        SerialNumberNonceGadget = SnNonceHGadget,
        SignatureGadget = SignatureSGadget,
        PRFGadget = PGadget,
    >,
    AddrC: CommitmentScheme,
    RecC: CommitmentScheme,
    LocalDataC: CommitmentScheme,
    SnNonceH: CRH,
    SignatureS: SignatureScheme,
    P: PRF,
    RecC::Output: Eq,
    AddrCGadget: CommitmentGadget<AddrC, C::InnerField>,
    RecCGadget: CommitmentGadget<RecC, C::InnerField>,
    LocalDataCGadget: CommitmentGadget<LocalDataC, C::InnerField>,
    SnNonceHGadget: CRHGadget<SnNonceH, C::InnerField>,
    SignatureSGadget: SignaturePublicKeyRandomizationGadget<SignatureS, C::InnerField>,
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
    // 3. local_data_comm_pp
    // 4. pred_vk_comm_pp
    // 5. sn_nonce_crh_pp.
    // 6. sig_pp.
    // 7. value_commitment_pp.
    // 8. ledger_parameters.
    // 9. ledger_digest.
    // 10. for i in 0..NUM_INPUT_RECORDS: old_serial_numbers[i].
    // 11. for j in 0..NUM_OUTPUT_RECORDS: new_commitments[i].
    // 12. predicate_comm.
    // 13. local_data_comm.
    // 14. binding_signature.
    let (addr_comm_pp, rec_comm_pp, pred_vk_comm_pp, local_data_comm_pp, sn_nonce_crh_pp, sig_pp, ledger_pp) = {
        let cs = &mut cs.ns(|| "Declare Comm and CRH parameters");
        let addr_comm_pp =
            AddrCGadget::ParametersGadget::alloc_input(&mut cs.ns(|| "Declare Addr Comm parameters"), || {
                Ok(comm_crh_sig_parameters.address_commitment_parameters.parameters())
            })?;

        let rec_comm_pp =
            RecCGadget::ParametersGadget::alloc_input(&mut cs.ns(|| "Declare Rec Comm parameters"), || {
                Ok(comm_crh_sig_parameters.record_commitment_parameters.parameters())
            })?;

        let local_data_comm_pp = LocalDataCGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare Local Data Comm parameters"),
            || Ok(comm_crh_sig_parameters.local_data_commitment_parameters.parameters()),
        )?;

        let pred_vk_comm_pp =
            <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<_, C::InnerField>>::ParametersGadget::alloc_input(
                &mut cs.ns(|| "Declare Pred Vk COMM parameters"),
                || Ok(comm_crh_sig_parameters.predicate_verification_key_commitment_parameters.parameters()),
            )?;

        let sn_nonce_crh_pp =
            SnNonceHGadget::ParametersGadget::alloc_input(&mut cs.ns(|| "Declare SN Nonce CRH parameters"), || {
                Ok(comm_crh_sig_parameters.serial_number_nonce_parameters.parameters())
            })?;

        let sig_pp = SignatureSGadget::ParametersGadget::alloc_input(&mut cs.ns(|| "Declare SIG Parameters"), || {
            Ok(&comm_crh_sig_parameters.signature_parameters)
        })?;
        //
        //        // TODO CHANGE THIS TO ALLOC_INPUT
        //        let value_commitment_pp = <C::ValueCommitmentGadget as CommitmentGadget<_, C::InnerField>>::ParametersGadget::alloc(&mut cs.ns(|| "Declare value commitment parameters"), || {
        //            Ok(&comm_crh_sig_parameters.value_commitment_parameters)
        //        })?;

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
            sig_pp,
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

            let given_payload = UInt8::alloc_vec(&mut declare_cs.ns(|| "Payload"), &record.payload().to_bytes())?;
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

        let (sk_prf, pk_sig) = {
            // Declare variables for addr_sk contents.
            let address_cs = &mut cs.ns(|| "Check address keypair");
            let pk_sig = SignatureSGadget::PublicKeyGadget::alloc(&mut address_cs.ns(|| "Declare pk_sig"), || {
                Ok(&secret_key.pk_sig)
            })?;

            let pk_sig_bytes = pk_sig.to_bytes(&mut address_cs.ns(|| "Pk_sig To Bytes"))?;

            let sk_prf = PGadget::new_seed(&mut address_cs.ns(|| "Declare sk_prf"), &secret_key.sk_prf);
            let metadata = UInt8::alloc_vec(&mut address_cs.ns(|| "Declare metadata"), &secret_key.metadata)?;
            let r_pk =
                AddrCGadget::RandomnessGadget::alloc(&mut address_cs.ns(|| "Declare r_pk"), || Ok(&secret_key.r_pk))?;

            let mut apk_input = pk_sig_bytes.clone();
            apk_input.extend_from_slice(&sk_prf);
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
            (sk_prf, pk_sig)
        };
        // ********************************************************************

        // ********************************************************************
        // Check that the serial number is derived correctly.
        // ********************************************************************
        let sn_nonce_bytes = {
            let sn_cs = &mut cs.ns(|| "Check that sn is derived correctly");

            let sn_nonce_bytes = sn_nonce.to_bytes(&mut sn_cs.ns(|| "Convert nonce to bytes"))?;

            let prf_seed = sk_prf;
            let randomizer = PGadget::check_evaluation_gadget(
                &mut sn_cs.ns(|| "Compute pk_sig randomizer"),
                &prf_seed,
                &sn_nonce_bytes,
            )?;
            let randomizer_bytes = randomizer.to_bytes(&mut sn_cs.ns(|| "Convert randomizer to bytes"))?;

            let candidate_sn = SignatureSGadget::check_randomization_gadget(
                &mut sn_cs.ns(|| "Compute serial number"),
                &sig_pp,
                &pk_sig,
                &randomizer_bytes,
            )?;

            let given_sn = SignatureSGadget::PublicKeyGadget::alloc_input(
                &mut sn_cs.ns(|| "Declare given serial number"),
                || Ok(given_serial_number),
            )?;

            candidate_sn.enforce_equal(
                &mut sn_cs.ns(|| "Check that given and computed serial numbers are equal"),
                &given_sn,
            )?;

            old_sns.push(candidate_sn);
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

            let given_payload = UInt8::alloc_vec(&mut declare_cs.ns(|| "Payload"), &record.payload().to_bytes())?;
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

        let local_data_comm_rand =
            LocalDataCGadget::RandomnessGadget::alloc(cs.ns(|| "Allocate local data commitment randomness"), || {
                Ok(local_data_rand)
            })?;

        let declared_local_data_comm =
            LocalDataCGadget::OutputGadget::alloc_input(cs.ns(|| "Allocate local data commitment"), || {
                Ok(local_data_comm)
            })?;

        let comm = LocalDataCGadget::check_commitment_gadget(
            cs.ns(|| "Commit to local data"),
            &local_data_comm_pp,
            &local_data_bytes,
            &local_data_comm_rand,
        )?;

        comm.enforce_equal(
            &mut cs.ns(|| "Check that local data commitment is valid"),
            &declared_local_data_comm,
        )?;

        // TODO Handle binding signature verification in the inner circuit
        let _binding_signature = UInt8::alloc_input_vec(&mut cs.ns(|| "Declare binding signature"), &to_bytes![
            binding_signature
        ]?)?;
    }
    Ok(())
}
