use crate::dpc::base_dpc::{
    binding_signature::{gadget_verification_setup, BindingSignature},
    parameters::CircuitParameters,
    record::DPCRecord,
    BaseDPCComponents,
};
use snarkos_algorithms::merkle_tree::{MerklePath, MerkleTreeDigest};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_gadgets::algorithms::merkle_tree::merkle_path::MerklePathGadget;
use snarkos_models::{
    algorithms::{CommitmentScheme, MerkleParameters, SignatureScheme, CRH, PRF},
    dpc::{DPCComponents, Record},
    gadgets::{
        algorithms::{
            BindingSignatureGadget,
            CRHGadget,
            CommitmentGadget,
            PRFGadget,
            SignaturePublicKeyRandomizationGadget,
        },
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, boolean::Boolean, eq::EqGadget, uint::UInt8, ToBytesGadget},
    },
};
use snarkos_objects::AccountPrivateKey;
use snarkos_utilities::{bytes::ToBytes, to_bytes};

pub fn execute_inner_proof_gadget<C: BaseDPCComponents, CS: ConstraintSystem<C::InnerField>>(
    cs: &mut CS,
    // Parameters
    circuit_parameters: &CircuitParameters<C>,
    ledger_parameters: &C::MerkleParameters,

    // Digest
    ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,

    // Old record stuff
    old_records: &[DPCRecord<C>],
    old_witnesses: &[MerklePath<C::MerkleParameters>],
    old_account_private_keys: &[AccountPrivateKey<C>],
    old_serial_numbers: &[<C::AccountSignature as SignatureScheme>::PublicKey],

    // New record stuff
    new_records: &[DPCRecord<C>],
    new_sn_nonce_randomness: &[[u8; 32]],
    new_commitments: &[<C::RecordCommitment as CommitmentScheme>::Output],

    // Rest
    predicate_commitment: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    predicate_randomness: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,

    local_data_commitment_leaves: &[<C::LocalDataCommitment as CommitmentScheme>::Output],
    local_data_commitment_leaves_randomness: &[<C::LocalDataCommitment as CommitmentScheme>::Randomness],
    local_data_witnesses: &[MerklePath<<C as DPCComponents>::LocalDataMerkleParameters>],
    local_data_commitment: &MerkleTreeDigest<<C as DPCComponents>::LocalDataMerkleParameters>,

    memo: &[u8; 32],
    input_value_commitments: &[<C::ValueCommitment as CommitmentScheme>::Output],
    input_value_commitment_randomness: &[<C::ValueCommitment as CommitmentScheme>::Randomness],
    output_value_commitments: &[<C::ValueCommitment as CommitmentScheme>::Output],
    output_value_commitment_randomness: &[<C::ValueCommitment as CommitmentScheme>::Randomness],
    value_balance: i64,
    binding_signature: &BindingSignature,
    network_id: u8,
) -> Result<(), SynthesisError> {
    base_dpc_execute_gadget_helper::<
        C,
        CS,
        C::AccountCommitment,
        C::AccountSignature,
        C::RecordCommitment,
        C::LocalDataCommitment,
        C::SerialNumberNonceCRH,
        C::PRF,
        C::AccountCommitmentGadget,
        C::AccountSignatureGadget,
        C::RecordCommitmentGadget,
        C::LocalDataCommitmentGadget,
        C::SerialNumberNonceCRHGadget,
        C::PRFGadget,
    >(
        cs,
        //
        circuit_parameters,
        ledger_parameters,
        //
        ledger_digest,
        //
        old_records,
        old_witnesses,
        old_account_private_keys,
        old_serial_numbers,
        //
        new_records,
        new_sn_nonce_randomness,
        new_commitments,
        //
        predicate_commitment,
        predicate_randomness,
        local_data_commitment_leaves,
        local_data_commitment_leaves_randomness,
        local_data_witnesses,
        local_data_commitment,
        memo,
        input_value_commitments,
        input_value_commitment_randomness,
        output_value_commitments,
        output_value_commitment_randomness,
        value_balance,
        binding_signature,
        network_id,
    )
}

fn base_dpc_execute_gadget_helper<
    C,
    CS: ConstraintSystem<C::InnerField>,
    AccountCommitment,
    AccountSignature,
    RecordCommitment,
    LocalDataCommitment,
    SerialNumberNonceCRH,
    P,
    AccountCommitmentGadget,
    AccountSignatureGadget,
    RecordCommitmentGadget,
    LocalDataCommitmentGadget,
    SerialNumberNonceCRHGadget,
    PGadget,
>(
    cs: &mut CS,

    //
    circuit_parameters: &CircuitParameters<C>,
    ledger_parameters: &C::MerkleParameters,

    //
    ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,

    //
    old_records: &[DPCRecord<C>],
    old_witnesses: &[MerklePath<C::MerkleParameters>],
    old_account_private_keys: &[AccountPrivateKey<C>],
    old_serial_numbers: &[AccountSignature::PublicKey],

    //
    new_records: &[DPCRecord<C>],
    new_sn_nonce_randomness: &[[u8; 32]],
    new_commitments: &[RecordCommitment::Output],

    //
    predicate_commitment: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    predicate_randomness: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,

    local_data_commitment_leaves: &[<C::LocalDataCommitment as CommitmentScheme>::Output],
    local_data_commitment_leaves_randomness: &[<C::LocalDataCommitment as CommitmentScheme>::Randomness],
    local_data_witnesses: &[MerklePath<<C as DPCComponents>::LocalDataMerkleParameters>],
    local_data_commitment: &MerkleTreeDigest<<C as DPCComponents>::LocalDataMerkleParameters>,

    memo: &[u8; 32],
    input_value_commitments: &[<C::ValueCommitment as CommitmentScheme>::Output],
    input_value_commitment_randomness: &[<C::ValueCommitment as CommitmentScheme>::Randomness],
    output_value_commitments: &[<C::ValueCommitment as CommitmentScheme>::Output],
    output_value_commitment_randomness: &[<C::ValueCommitment as CommitmentScheme>::Randomness],
    value_balance: i64,
    binding_signature: &BindingSignature,
    network_id: u8,
) -> Result<(), SynthesisError>
where
    C: BaseDPCComponents<
        AccountCommitment = AccountCommitment,
        AccountSignature = AccountSignature,
        RecordCommitment = RecordCommitment,
        LocalDataCommitment = LocalDataCommitment,
        SerialNumberNonceCRH = SerialNumberNonceCRH,
        PRF = P,
        AccountCommitmentGadget = AccountCommitmentGadget,
        AccountSignatureGadget = AccountSignatureGadget,
        RecordCommitmentGadget = RecordCommitmentGadget,
        LocalDataCommitmentGadget = LocalDataCommitmentGadget,
        SerialNumberNonceCRHGadget = SerialNumberNonceCRHGadget,
        PRFGadget = PGadget,
    >,
    AccountCommitment: CommitmentScheme,
    AccountSignature: SignatureScheme,
    RecordCommitment: CommitmentScheme,
    LocalDataCommitment: CommitmentScheme,
    SerialNumberNonceCRH: CRH,
    P: PRF,
    RecordCommitment::Output: Eq,
    AccountCommitmentGadget: CommitmentGadget<AccountCommitment, C::InnerField>,
    AccountSignatureGadget: SignaturePublicKeyRandomizationGadget<AccountSignature, C::InnerField>,
    RecordCommitmentGadget: CommitmentGadget<RecordCommitment, C::InnerField>,
    LocalDataCommitmentGadget: CommitmentGadget<LocalDataCommitment, C::InnerField>,
    SerialNumberNonceCRHGadget: CRHGadget<SerialNumberNonceCRH, C::InnerField>,
    PGadget: PRFGadget<P, C::InnerField>,
{
    let mut old_serial_numbers_gadgets = Vec::with_capacity(old_records.len());
    let mut old_serial_numbers_bytes_gadgets = Vec::with_capacity(old_records.len() * 32); // Serial numbers are 32 bytes
    let mut old_record_commitments_gadgets = Vec::with_capacity(old_records.len());
    let mut old_account_public_keys_gadgets = Vec::with_capacity(old_records.len());
    let mut old_dummy_flags_gadgets = Vec::with_capacity(old_records.len());
    let mut old_value_gadgets = Vec::with_capacity(old_records.len());
    let mut old_payloads_gadgets = Vec::with_capacity(old_records.len());
    let mut old_birth_predicate_hashes_gadgets = Vec::with_capacity(old_records.len());
    let mut old_death_predicate_hashes_gadgets = Vec::with_capacity(old_records.len());

    let mut new_record_commitments_gadgets = Vec::with_capacity(new_records.len());
    let mut new_account_public_keys_gadgets = Vec::with_capacity(new_records.len());
    let mut new_dummy_flags_gadgets = Vec::with_capacity(new_records.len());
    let mut new_value_gadgets = Vec::with_capacity(new_records.len());
    let mut new_payloads_gadgets = Vec::with_capacity(new_records.len());
    let mut new_birth_predicate_hashes_gadgets = Vec::with_capacity(new_records.len());
    let mut new_death_predicate_hashes_gadgets = Vec::with_capacity(new_records.len());

    // Order for allocation of input:
    // 1. account_commitment_parameters
    // 2. account_signature_parameters
    // 3. record_commitment_parameters
    // 4. predicate_vk_commitment_parameters
    // 5. local_data_commitment_parameters
    // 6. local_data_commitment_parameters
    // 7. serial_number_nonce_crh_parameters
    // 8. value_commitment_parameters
    // 9. ledger_parameters
    // 10. ledger_digest
    // 11. for i in 0..NUM_INPUT_RECORDS: old_serial_numbers[i]
    // 12. for j in 0..NUM_OUTPUT_RECORDS: new_commitments[i]
    // 13. predicate_commitment
    // 14. local_data_commitment
    // 15. binding_signature
    let (
        account_commitment_parameters,
        account_signature_parameters,
        record_commitment_parameters,
        predicate_vk_commitment_parameters,
        local_data_commitment_parameters,
        local_data_merkle_tree_parameters,
        serial_number_nonce_crh_parameters,
        value_commitment_parameters,
        ledger_parameters,
    ) = {
        let cs = &mut cs.ns(|| "Declare commitment and CRH parameters");

        let account_commitment_parameters = AccountCommitmentGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare account commit parameters"),
            || Ok(circuit_parameters.account_commitment.parameters()),
        )?;

        let account_signature_parameters = AccountSignatureGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare account signature parameters"),
            || Ok(circuit_parameters.account_signature.parameters()),
        )?;

        let record_commitment_parameters = RecordCommitmentGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare record commitment parameters"),
            || Ok(circuit_parameters.record_commitment.parameters()),
        )?;

        let predicate_vk_commitment_parameters = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<
            _,
            C::InnerField,
        >>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare predicate vk commitment parameters"),
            || Ok(circuit_parameters.predicate_verification_key_commitment.parameters()),
        )?;

        let local_data_commitment_parameters = LocalDataCommitmentGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare local data commitment parameters"),
            || Ok(circuit_parameters.local_data_commitment.parameters()),
        )?;

        let local_data_merkle_tree_parameters =
            <<C as DPCComponents>::LocalDataMerkleHashGadget as CRHGadget<_, _>>::ParametersGadget::alloc_input(
                &mut cs.ns(|| "Declare local data merkle tree parameters"),
                || Ok(circuit_parameters.local_data_merkle_tree.parameters()),
            )?;

        let serial_number_nonce_crh_parameters = SerialNumberNonceCRHGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare serial number nonce CRH parameters"),
            || Ok(circuit_parameters.serial_number_nonce.parameters()),
        )?;

        let value_commitment_parameters =
            <C::ValueCommitmentGadget as CommitmentGadget<_, _>>::ParametersGadget::alloc_input(
                &mut cs.ns(|| "Declare value commitment parameters"),
                || Ok(circuit_parameters.value_commitment.parameters()),
            )?;

        let ledger_parameters = <C::MerkleHashGadget as CRHGadget<_, _>>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare ledger parameters"),
            || Ok(ledger_parameters.parameters()),
        )?;

        (
            account_commitment_parameters,
            account_signature_parameters,
            record_commitment_parameters,
            predicate_vk_commitment_parameters,
            local_data_commitment_parameters,
            local_data_merkle_tree_parameters,
            serial_number_nonce_crh_parameters,
            value_commitment_parameters,
            ledger_parameters,
        )
    };

    let digest_gadget = <C::MerkleHashGadget as CRHGadget<_, _>>::OutputGadget::alloc_input(
        &mut cs.ns(|| "Declare ledger digest"),
        || Ok(ledger_digest),
    )?;

    for (i, (((record, witness), account_private_key), given_serial_number)) in old_records
        .iter()
        .zip(old_witnesses)
        .zip(old_account_private_keys)
        .zip(old_serial_numbers)
        .enumerate()
    {
        let cs = &mut cs.ns(|| format!("Process input record {}", i));

        // Declare record contents
        let (
            given_account_public_key,
            given_commitment,
            given_is_dummy,
            given_value,
            given_payload,
            given_birth_predicate_crh,
            given_death_predicate_crh,
            given_commitment_randomness,
            serial_number_nonce,
        ) = {
            let declare_cs = &mut cs.ns(|| "Declare input record");

            // No need to check that commitments, public keys and hashes are in
            // prime order subgroup because the commitment and CRH parameters
            // are trusted, and so when we recompute these, the newly computed
            // values will always be in correct subgroup. If the input cm, pk
            // or hash is incorrect, then it will not match the computed equivalent.
            let given_account_public_key = AccountCommitmentGadget::OutputGadget::alloc(
                &mut declare_cs.ns(|| "given_account_public_key"),
                || Ok(&record.account_public_key().commitment),
            )?;
            old_account_public_keys_gadgets.push(given_account_public_key.clone());

            let given_commitment =
                RecordCommitmentGadget::OutputGadget::alloc(&mut declare_cs.ns(|| "given_commitment"), || {
                    Ok(record.commitment().clone())
                })?;
            old_record_commitments_gadgets.push(given_commitment.clone());

            let given_is_dummy = Boolean::alloc(&mut declare_cs.ns(|| "given_is_dummy"), || Ok(record.is_dummy()))?;
            old_dummy_flags_gadgets.push(given_is_dummy.clone());

            let given_value = UInt8::alloc_vec(&mut declare_cs.ns(|| "given_value"), &to_bytes![record.value()]?)?;
            old_value_gadgets.push(given_value.clone());

            let given_payload = UInt8::alloc_vec(&mut declare_cs.ns(|| "given_payload"), &record.payload().to_bytes())?;
            old_payloads_gadgets.push(given_payload.clone());

            let given_birth_predicate_crh = UInt8::alloc_vec(
                &mut declare_cs.ns(|| "given_birth_predicate_crh"),
                &record.birth_predicate_repr(),
            )?;
            old_birth_predicate_hashes_gadgets.push(given_birth_predicate_crh.clone());

            let given_death_predicate_crh = UInt8::alloc_vec(
                &mut declare_cs.ns(|| "given_death_predicate_crh"),
                &record.death_predicate_repr(),
            )?;
            old_death_predicate_hashes_gadgets.push(given_death_predicate_crh.clone());

            let given_commitment_randomness = RecordCommitmentGadget::RandomnessGadget::alloc(
                &mut declare_cs.ns(|| "given_commitment_randomness"),
                || Ok(record.commitment_randomness()),
            )?;

            let serial_number_nonce =
                SerialNumberNonceCRHGadget::OutputGadget::alloc(&mut declare_cs.ns(|| "serial_number_nonce"), || {
                    Ok(record.serial_number_nonce())
                })?;
            (
                given_account_public_key,
                given_commitment,
                given_is_dummy,
                given_value,
                given_payload,
                given_birth_predicate_crh,
                given_death_predicate_crh,
                given_commitment_randomness,
                serial_number_nonce,
            )
        };

        // ********************************************************************
        // Check that the commitment appears on the ledger,
        // i.e., the membership witness is valid with respect to the
        // transaction set digest.
        // ********************************************************************
        {
            let witness_cs = &mut cs.ns(|| "Check ledger membership witness");

            let witness_gadget = MerklePathGadget::<_, C::MerkleHashGadget, _>::alloc(
                &mut witness_cs.ns(|| "Declare membership witness"),
                || Ok(witness),
            )?;

            witness_gadget.conditionally_check_membership(
                &mut witness_cs.ns(|| "Perform ledger membership witness check"),
                &ledger_parameters,
                &digest_gadget,
                &given_commitment,
                &given_is_dummy.not(),
            )?;
        }
        // ********************************************************************

        // ********************************************************************
        // Check that the account public key and private key form a valid key
        // pair.
        // ********************************************************************

        let (sk_prf, pk_sig) = {
            // Declare variables for account contents.
            let account_cs = &mut cs.ns(|| "Check account");

            let pk_sig_native = account_private_key
                .pk_sig(&circuit_parameters.account_signature)
                .map_err(|_| SynthesisError::AssignmentMissing)?;
            let pk_sig =
                AccountSignatureGadget::PublicKeyGadget::alloc(&mut account_cs.ns(|| "Declare pk_sig"), || {
                    Ok(&pk_sig_native)
                })?;

            let pk_sig_bytes = pk_sig.to_bytes(&mut account_cs.ns(|| "pk_sig to_bytes"))?;

            let sk_prf = PGadget::new_seed(&mut account_cs.ns(|| "Declare sk_prf"), &account_private_key.sk_prf);
            let metadata = UInt8::alloc_vec(&mut account_cs.ns(|| "Declare metadata"), &account_private_key.metadata)?;
            let r_pk = AccountCommitmentGadget::RandomnessGadget::alloc(&mut account_cs.ns(|| "Declare r_pk"), || {
                Ok(&account_private_key.r_pk)
            })?;

            let mut account_public_key_input = pk_sig_bytes.clone();
            account_public_key_input.extend_from_slice(&sk_prf);
            account_public_key_input.extend_from_slice(&metadata);

            let candidate_account_public_key = AccountCommitmentGadget::check_commitment_gadget(
                &mut account_cs.ns(|| "Compute account public key"),
                &account_commitment_parameters,
                &account_public_key_input,
                &r_pk,
            )?;

            candidate_account_public_key.enforce_equal(
                &mut account_cs.ns(|| "Check that declared and computed public keys are equal"),
                &given_account_public_key,
            )?;
            (sk_prf, pk_sig)
        };
        // ********************************************************************

        // ********************************************************************
        // Check that the serial number is derived correctly.
        // ********************************************************************
        let serial_number_nonce_bytes = {
            let sn_cs = &mut cs.ns(|| "Check that sn is derived correctly");

            let serial_number_nonce_bytes = serial_number_nonce.to_bytes(&mut sn_cs.ns(|| "Convert nonce to bytes"))?;

            let prf_seed = sk_prf;
            let randomizer = PGadget::check_evaluation_gadget(
                &mut sn_cs.ns(|| "Compute pk_sig randomizer"),
                &prf_seed,
                &serial_number_nonce_bytes,
            )?;
            let randomizer_bytes = randomizer.to_bytes(&mut sn_cs.ns(|| "Convert randomizer to bytes"))?;

            let candidate_serial_number_gadget = AccountSignatureGadget::check_randomization_gadget(
                &mut sn_cs.ns(|| "Compute serial number"),
                &account_signature_parameters,
                &pk_sig,
                &randomizer_bytes,
            )?;

            let given_serial_number_gadget = AccountSignatureGadget::PublicKeyGadget::alloc_input(
                &mut sn_cs.ns(|| "Declare given serial number"),
                || Ok(given_serial_number),
            )?;

            candidate_serial_number_gadget.enforce_equal(
                &mut sn_cs.ns(|| "Check that given and computed serial numbers are equal"),
                &given_serial_number_gadget,
            )?;

            old_serial_numbers_gadgets.push(candidate_serial_number_gadget.clone());

            // Convert input serial numbers to bytes
            {
                let bytes = candidate_serial_number_gadget
                    .to_bytes(&mut sn_cs.ns(|| format!("Convert {}-th serial number to bytes", i)))?;
                old_serial_numbers_bytes_gadgets.extend_from_slice(&bytes);
            }

            serial_number_nonce_bytes
        };
        // ********************************************************************

        // ********************************************************************
        // Check that the value commitment is correct
        // ********************************************************************
        {
            let vc_cs = &mut cs.ns(|| "Check that the value commitment is correct");

            let value_commitment_randomness_gadget =
                <C::ValueCommitmentGadget as CommitmentGadget<_, _>>::RandomnessGadget::alloc(
                    vc_cs.ns(|| "Allocate value commitment randomness"),
                    || Ok(&input_value_commitment_randomness[i]),
                )?;

            let declared_value_commitment_gadget =
                <C::ValueCommitmentGadget as CommitmentGadget<_, _>>::OutputGadget::alloc(
                    vc_cs.ns(|| "Allocate declared value commitment"),
                    || Ok(&input_value_commitments[i]),
                )?;

            let computed_value_commitment_gadget = C::ValueCommitmentGadget::check_commitment_gadget(
                vc_cs.ns(|| "Generate value commitment"),
                &value_commitment_parameters,
                &given_value,
                &value_commitment_randomness_gadget,
            )?;

            // Check that the value commitments are equivalent
            computed_value_commitment_gadget.enforce_equal(
                &mut vc_cs.ns(|| "Check that declared and computed value commitments are equal"),
                &declared_value_commitment_gadget,
            )?;
        };
        // ********************************************************************

        // *******************************************************************
        // Check that the record is well-formed.
        // *******************************************************************
        {
            let commitment_cs = &mut cs.ns(|| "Check that record is well-formed");

            let account_public_key_bytes =
                given_account_public_key.to_bytes(&mut commitment_cs.ns(|| "Convert account_public_key to bytes"))?;
            let is_dummy_bytes = given_is_dummy.to_bytes(&mut commitment_cs.ns(|| "Convert is_dummy to bytes"))?;

            let mut commitment_input = Vec::new();
            commitment_input.extend_from_slice(&account_public_key_bytes);
            commitment_input.extend_from_slice(&is_dummy_bytes);
            commitment_input.extend_from_slice(&given_value);
            commitment_input.extend_from_slice(&given_payload);
            commitment_input.extend_from_slice(&given_birth_predicate_crh);
            commitment_input.extend_from_slice(&given_death_predicate_crh);
            commitment_input.extend_from_slice(&serial_number_nonce_bytes);

            let candidate_commitment = RecordCommitmentGadget::check_commitment_gadget(
                &mut commitment_cs.ns(|| "Compute commitment"),
                &record_commitment_parameters,
                &commitment_input,
                &given_commitment_randomness,
            )?;

            candidate_commitment.enforce_equal(
                &mut commitment_cs.ns(|| "Check that declared and computed commitments are equal"),
                &given_commitment,
            )?;
        }
    }

    for (j, ((record, sn_nonce_randomness), commitment)) in new_records
        .iter()
        .zip(new_sn_nonce_randomness)
        .zip(new_commitments)
        .enumerate()
    {
        let cs = &mut cs.ns(|| format!("Process output record {}", j));

        let (
            given_account_public_key,
            given_record_commitment,
            given_commitment,
            given_is_dummy,
            given_value,
            given_payload,
            given_birth_predicate_hash,
            given_death_predicate_hash,
            given_commitment_randomness,
            serial_number_nonce,
        ) = {
            let declare_cs = &mut cs.ns(|| "Declare output record");

            let given_account_public_key = AccountCommitmentGadget::OutputGadget::alloc(
                &mut declare_cs.ns(|| "given_account_public_key"),
                || Ok(&record.account_public_key().commitment),
            )?;
            new_account_public_keys_gadgets.push(given_account_public_key.clone());

            let given_record_commitment =
                RecordCommitmentGadget::OutputGadget::alloc(&mut declare_cs.ns(|| "given_record_commitment"), || {
                    Ok(record.commitment())
                })?;
            new_record_commitments_gadgets.push(given_record_commitment.clone());

            let given_commitment =
                RecordCommitmentGadget::OutputGadget::alloc_input(&mut declare_cs.ns(|| "given_commitment"), || {
                    Ok(commitment)
                })?;

            let given_is_dummy = Boolean::alloc(&mut declare_cs.ns(|| "given_is_dummy"), || Ok(record.is_dummy()))?;
            new_dummy_flags_gadgets.push(given_is_dummy.clone());

            let given_value = UInt8::alloc_vec(&mut declare_cs.ns(|| "given_value"), &to_bytes![record.value()]?)?;
            new_value_gadgets.push(given_value.clone());

            let given_payload = UInt8::alloc_vec(&mut declare_cs.ns(|| "given_payload"), &record.payload().to_bytes())?;
            new_payloads_gadgets.push(given_payload.clone());

            let given_birth_predicate_hash = UInt8::alloc_vec(
                &mut declare_cs.ns(|| "given_birth_predicate_hash"),
                &record.birth_predicate_repr(),
            )?;
            new_birth_predicate_hashes_gadgets.push(given_birth_predicate_hash.clone());

            let given_death_predicate_hash = UInt8::alloc_vec(
                &mut declare_cs.ns(|| "given_death_predicate_hash"),
                &record.death_predicate_repr(),
            )?;
            new_death_predicate_hashes_gadgets.push(given_death_predicate_hash.clone());

            let given_commitment_randomness = RecordCommitmentGadget::RandomnessGadget::alloc(
                &mut declare_cs.ns(|| "given_commitment_randomness"),
                || Ok(record.commitment_randomness()),
            )?;

            let serial_number_nonce =
                SerialNumberNonceCRHGadget::OutputGadget::alloc(&mut declare_cs.ns(|| "serial_number_nonce"), || {
                    Ok(record.serial_number_nonce())
                })?;

            (
                given_account_public_key,
                given_record_commitment,
                given_commitment,
                given_is_dummy,
                given_value,
                given_payload,
                given_birth_predicate_hash,
                given_death_predicate_hash,
                given_commitment_randomness,
                serial_number_nonce,
            )
        };
        // ********************************************************************

        // *******************************************************************
        // Check that the serial number nonce is computed correctly.
        // *******************************************************************
        {
            let sn_cs = &mut cs.ns(|| "Check that serial number nonce is computed correctly");

            let current_record_number = UInt8::constant(j as u8);
            let mut current_record_number_bytes_le = vec![current_record_number];

            let serial_number_nonce_randomness = UInt8::alloc_vec(
                sn_cs.ns(|| "Allocate serial number nonce randomness"),
                sn_nonce_randomness,
            )?;
            current_record_number_bytes_le.extend_from_slice(&serial_number_nonce_randomness);
            current_record_number_bytes_le.extend_from_slice(&old_serial_numbers_bytes_gadgets);

            let sn_nonce_input = current_record_number_bytes_le;

            let candidate_sn_nonce = SerialNumberNonceCRHGadget::check_evaluation_gadget(
                &mut sn_cs.ns(|| "Compute serial number nonce"),
                &serial_number_nonce_crh_parameters,
                &sn_nonce_input,
            )?;
            candidate_sn_nonce.enforce_equal(
                &mut sn_cs.ns(|| "Check that computed nonce matches provided nonce"),
                &serial_number_nonce,
            )?;
        }
        // ********************************************************************

        // ********************************************************************
        // Check that the value commitment is correct
        // ********************************************************************
        {
            let vc_cs = &mut cs.ns(|| "Check that the value commitment is correct");

            let value_commitment_randomness_gadget =
                <C::ValueCommitmentGadget as CommitmentGadget<_, _>>::RandomnessGadget::alloc(
                    vc_cs.ns(|| "Allocate value commitment randomness"),
                    || Ok(&output_value_commitment_randomness[j]),
                )?;

            let declared_value_commitment_gadget =
                <C::ValueCommitmentGadget as CommitmentGadget<_, _>>::OutputGadget::alloc(
                    vc_cs.ns(|| "Allocate declared value commitment"),
                    || Ok(&output_value_commitments[j]),
                )?;

            let computed_value_commitment_gadget = C::ValueCommitmentGadget::check_commitment_gadget(
                vc_cs.ns(|| "Generate value commitment"),
                &value_commitment_parameters,
                &given_value,
                &value_commitment_randomness_gadget,
            )?;

            // Check that the value commitments are equivalent
            computed_value_commitment_gadget.enforce_equal(
                &mut vc_cs.ns(|| "Check that declared and computed value commitments are equal"),
                &declared_value_commitment_gadget,
            )?;
        };
        // *******************************************************************

        // *******************************************************************
        // Check that the record is well-formed.
        // *******************************************************************
        {
            let commitment_cs = &mut cs.ns(|| "Check that record is well-formed");

            let account_public_key_bytes =
                given_account_public_key.to_bytes(&mut commitment_cs.ns(|| "Convert account_public_key to bytes"))?;
            let is_dummy_bytes = given_is_dummy.to_bytes(&mut commitment_cs.ns(|| "Convert is_dummy to bytes"))?;
            let sn_nonce_bytes = serial_number_nonce.to_bytes(&mut commitment_cs.ns(|| "Convert sn nonce to bytes"))?;

            let mut commitment_input = Vec::new();
            commitment_input.extend_from_slice(&account_public_key_bytes);
            commitment_input.extend_from_slice(&is_dummy_bytes);
            commitment_input.extend_from_slice(&given_value);
            commitment_input.extend_from_slice(&given_payload);
            commitment_input.extend_from_slice(&given_birth_predicate_hash);
            commitment_input.extend_from_slice(&given_death_predicate_hash);
            commitment_input.extend_from_slice(&sn_nonce_bytes);

            let candidate_commitment = RecordCommitmentGadget::check_commitment_gadget(
                &mut commitment_cs.ns(|| "Compute record commitment"),
                &record_commitment_parameters,
                &commitment_input,
                &given_commitment_randomness,
            )?;
            candidate_commitment.enforce_equal(
                &mut commitment_cs.ns(|| "Check that computed commitment matches pub input"),
                &given_commitment,
            )?;
            candidate_commitment.enforce_equal(
                &mut commitment_cs.ns(|| "Check that computed commitment matches declared comm"),
                &given_record_commitment,
            )?;
        }
    }
    // *******************************************************************

    // *******************************************************************
    // Check that predicate commitment is well formed.
    // *******************************************************************
    {
        let commitment_cs = &mut cs.ns(|| "Check that predicate commitment is well-formed");

        let mut input = Vec::new();
        for i in 0..C::NUM_INPUT_RECORDS {
            input.extend_from_slice(&old_death_predicate_hashes_gadgets[i]);
        }

        for j in 0..C::NUM_OUTPUT_RECORDS {
            input.extend_from_slice(&new_birth_predicate_hashes_gadgets[j]);
        }

        let given_commitment_randomness = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<
            _,
            C::InnerField,
        >>::RandomnessGadget::alloc(
            &mut commitment_cs.ns(|| "given_commitment_randomness"),
            || Ok(predicate_randomness),
        )?;

        let given_commitment = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<_, C::InnerField>>::OutputGadget::alloc_input(
            &mut commitment_cs.ns(|| "given_commitment"),
            || Ok(predicate_commitment),
        )?;

        let candidate_commitment = <C::PredicateVerificationKeyCommitmentGadget as CommitmentGadget<
            _,
            C::InnerField,
        >>::check_commitment_gadget(
            &mut commitment_cs.ns(|| "candidate_commitment"),
            &predicate_vk_commitment_parameters,
            &input,
            &given_commitment_randomness,
        )?;

        candidate_commitment.enforce_equal(
            &mut commitment_cs.ns(|| "Check that declared and computed commitments are equal"),
            &given_commitment,
        )?;
    }
    // ********************************************************************

    // ********************************************************************
    // Check that the local data commitments and digest is valid
    // ********************************************************************
    {
        let mut cs = cs.ns(|| "Check that local data commitment is valid.");

        let mut given_local_data_commitment_leaves = vec![];
        for (i, commtiment) in local_data_commitment_leaves.iter().enumerate() {
            let given_commitment = LocalDataCommitmentGadget::OutputGadget::alloc(
                &mut cs.ns(|| format!("Local data commitment leaf: {:?}", i)),
                || Ok(commtiment),
            )?;

            given_local_data_commitment_leaves.push(given_commitment);
        }

        let mut given_local_data_commitment_leaves_randomness = vec![];
        for (i, randomness) in local_data_commitment_leaves_randomness.iter().enumerate() {
            let given_randomness = LocalDataCommitmentGadget::RandomnessGadget::alloc(
                &mut cs.ns(|| format!("Local data commitment randomness: {:?}", i)),
                || Ok(randomness),
            )?;

            given_local_data_commitment_leaves_randomness.push(given_randomness);
        }

        let mut index = 0;

        for i in 0..C::NUM_INPUT_RECORDS {
            let mut cs = cs.ns(|| format!("Construct local data commitment with input record {}", i));
            let mut input_record_bytes = vec![];
            input_record_bytes.extend_from_slice(
                &old_record_commitments_gadgets[i].to_bytes(&mut cs.ns(|| "old_record_commitment"))?,
            );
            input_record_bytes.extend_from_slice(
                &old_account_public_keys_gadgets[i].to_bytes(&mut cs.ns(|| "old_account_public_key"))?,
            );
            input_record_bytes.extend_from_slice(&old_dummy_flags_gadgets[i].to_bytes(&mut cs.ns(|| "is_dummy"))?);
            input_record_bytes.extend_from_slice(&old_value_gadgets[i]);
            input_record_bytes.extend_from_slice(&old_payloads_gadgets[i]);
            input_record_bytes.extend_from_slice(&old_birth_predicate_hashes_gadgets[i]);
            input_record_bytes.extend_from_slice(&old_death_predicate_hashes_gadgets[i]);
            input_record_bytes
                .extend_from_slice(&old_serial_numbers_gadgets[i].to_bytes(&mut cs.ns(|| "old_serial_number"))?);

            let commitment = LocalDataCommitmentGadget::check_commitment_gadget(
                cs.ns(|| format!("Commit input record {}", i)),
                &local_data_commitment_parameters,
                &input_record_bytes,
                &given_local_data_commitment_leaves_randomness[index],
            )?;

            commitment.enforce_equal(
                &mut cs.ns(|| format!("Check that local data commitment is valid - leaf {}", index)),
                &given_local_data_commitment_leaves[index],
            )?;

            index += 1;
        }

        for j in 0..C::NUM_OUTPUT_RECORDS {
            let mut cs = cs.ns(|| format!("Construct local data with output record {}", j));
            let mut output_record_bytes = vec![];
            output_record_bytes
                .extend_from_slice(&new_record_commitments_gadgets[j].to_bytes(&mut cs.ns(|| "record_commitment"))?);
            output_record_bytes
                .extend_from_slice(&new_account_public_keys_gadgets[j].to_bytes(&mut cs.ns(|| "account_public_key"))?);
            output_record_bytes.extend_from_slice(&new_dummy_flags_gadgets[j].to_bytes(&mut cs.ns(|| "is_dummy"))?);
            output_record_bytes.extend_from_slice(&new_value_gadgets[j]);
            output_record_bytes.extend_from_slice(&new_payloads_gadgets[j]);
            output_record_bytes.extend_from_slice(&new_birth_predicate_hashes_gadgets[j]);
            output_record_bytes.extend_from_slice(&new_death_predicate_hashes_gadgets[j]);

            let commitment = LocalDataCommitmentGadget::check_commitment_gadget(
                cs.ns(|| format!("Commit output record {}", j)),
                &local_data_commitment_parameters,
                &output_record_bytes,
                &given_local_data_commitment_leaves_randomness[index],
            )?;

            commitment.enforce_equal(
                &mut cs.ns(|| format!("Check that local data commitment is valid - leaf {}", index)),
                &given_local_data_commitment_leaves[index],
            )?;

            index += 1;
        }

        let memo = UInt8::alloc_input_vec(cs.ns(|| "Allocate memorandum"), memo)?;

        let memo_commitment = LocalDataCommitmentGadget::check_commitment_gadget(
            cs.ns(|| "Commit memo"),
            &local_data_commitment_parameters,
            &memo,
            &given_local_data_commitment_leaves_randomness[index],
        )?;

        memo_commitment.enforce_equal(
            &mut cs.ns(|| format!("Check that local data commitment is valid - leaf {}", index)),
            &given_local_data_commitment_leaves[index],
        )?;

        index += 1;

        let network_id = UInt8::alloc_input_vec(cs.ns(|| "Allocate network id"), &[network_id])?;

        let network_id_commitment = LocalDataCommitmentGadget::check_commitment_gadget(
            cs.ns(|| "Commit network id"),
            &local_data_commitment_parameters,
            &network_id,
            &given_local_data_commitment_leaves_randomness[index],
        )?;

        network_id_commitment.enforce_equal(
            &mut cs.ns(|| format!("Check that local data commitment is valid - leaf {}", index)),
            &given_local_data_commitment_leaves[index],
        )?;

        let local_data_commitment_gadget =
            <<C as DPCComponents>::LocalDataMerkleHashGadget as CRHGadget<_, _>>::OutputGadget::alloc_input(
                &mut cs.ns(|| "Declare local data commitment digest"),
                || Ok(local_data_commitment),
            )?;

        let witness_cs = &mut cs.ns(|| "Check local data merkle tree membership witness");

        for (i, witness) in local_data_witnesses.iter().enumerate() {
            let witness_gadget = MerklePathGadget::<_, <C as DPCComponents>::LocalDataMerkleHashGadget, _>::alloc(
                &mut witness_cs.ns(|| format!("Declare local data membership witness {}", i)),
                || Ok(witness),
            )?;

            witness_gadget.check_membership(
                &mut witness_cs.ns(|| format!("Perform ledger membership witness check: {}", i)),
                &local_data_merkle_tree_parameters,
                &local_data_commitment_gadget,
                &given_local_data_commitment_leaves[i],
            )?;
        }
    }
    // *******************************************************************

    // *******************************************************************
    // Check that the binding signature is valid
    // *******************************************************************
    {
        let mut cs = cs.ns(|| "Check that the binding signature is valid.");

        let (c, partial_bvk, affine_r, recommit) =
            gadget_verification_setup::<C::ValueCommitment, C::BindingSignatureGroup>(
                &circuit_parameters.value_commitment,
                &input_value_commitments,
                &output_value_commitments,
                &to_bytes![local_data_commitment]?,
                &binding_signature,
            )
            .unwrap();

        let binding_signature_parameters = <C::BindingSignatureGadget as BindingSignatureGadget<
            _,
            C::InnerField,
            C::BindingSignatureGroup,
        >>::ParametersGadget::alloc(
            &mut cs.ns(|| "Declare value commitment parameters as binding signature parameters"),
            || Ok(circuit_parameters.value_commitment.parameters()),
        )?;

        let c_gadget = <C::BindingSignatureGadget as BindingSignatureGadget<
            _,
            C::InnerField,
            C::BindingSignatureGroup,
        >>::RandomnessGadget::alloc(&mut cs.ns(|| "c_gadget"), || Ok(c))?;

        let partial_bvk_gadget =
            <C::BindingSignatureGadget as BindingSignatureGadget<
                C::ValueCommitment,
                C::InnerField,
                C::BindingSignatureGroup,
            >>::OutputGadget::alloc(&mut cs.ns(|| "partial_bvk_gadget"), || Ok(partial_bvk))?;

        let affine_r_gadget = <C::BindingSignatureGadget as BindingSignatureGadget<
            C::ValueCommitment,
            C::InnerField,
            C::BindingSignatureGroup,
        >>::OutputGadget::alloc(&mut cs.ns(|| "affine_r_gadget"), || Ok(affine_r))?;

        let recommit_gadget = <C::BindingSignatureGadget as BindingSignatureGadget<
            C::ValueCommitment,
            C::InnerField,
            C::BindingSignatureGroup,
        >>::OutputGadget::alloc(&mut cs.ns(|| "recommit_gadget"), || Ok(recommit))?;

        let value_balance_bytes = UInt8::alloc_input_vec(
            cs.ns(|| "value_balance_bytes"),
            &(value_balance.abs() as u64).to_le_bytes(),
        )?;

        let is_negative = Boolean::alloc_input(&mut cs.ns(|| "value_balance_is_negative"), || {
            Ok(value_balance.is_negative())
        })?;

        let value_balance_commitment = <C::BindingSignatureGadget as BindingSignatureGadget<
            _,
            C::InnerField,
            C::BindingSignatureGroup,
        >>::check_value_balance_commitment_gadget(
            &mut cs.ns(|| "value_balance_commitment"),
            &binding_signature_parameters,
            &value_balance_bytes,
        )?;

        <C::BindingSignatureGadget as BindingSignatureGadget<_, C::InnerField, C::BindingSignatureGroup>>::check_binding_signature_gadget(
            &mut cs.ns(|| "verify_binding_signature"),
            &partial_bvk_gadget,
            &value_balance_commitment,
            &is_negative,
            &c_gadget,
            &affine_r_gadget,
            &recommit_gadget,
        )?;
    }

    Ok(())
}
