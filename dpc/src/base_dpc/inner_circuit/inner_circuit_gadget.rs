use crate::base_dpc::{
    parameters::SystemParameters,
    record_encryption::RecordEncryptionGadgetComponents,
    records::DPCRecord,
    BaseDPCComponents,
};
use snarkos_algorithms::merkle_tree::{MerklePath, MerkleTreeDigest};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_gadgets::algorithms::{encoding::Elligator2FieldGadget, merkle_tree::merkle_path::MerklePathGadget};
use snarkos_models::{
    algorithms::{CommitmentScheme, EncryptionScheme, MerkleParameters, SignatureScheme, CRH, PRF},
    curves::{
        AffineCurve,
        Field,
        Group,
        MontgomeryModelParameters,
        One,
        PrimeField,
        ProjectiveCurve,
        TEModelParameters,
    },
    dpc::Record,
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget, EncryptionGadget, PRFGadget, SignaturePublicKeyRandomizationGadget},
        curves::FieldGadget,
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            arithmetic::{add::Add, sub::Sub},
            boolean::Boolean,
            eq::{ConditionalEqGadget, EqGadget, EvaluateEqGadget},
            int::{Int, Int64},
            uint::UInt8,
            ToBitsGadget,
            ToBytesGadget,
        },
    },
};
use snarkos_objects::AccountPrivateKey;
use snarkos_utilities::{
    bits_to_bytes,
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use std::ops::Mul;

pub fn execute_inner_proof_gadget<C: BaseDPCComponents, CS: ConstraintSystem<C::InnerField>>(
    cs: &mut CS,
    // Parameters
    system_parameters: &SystemParameters<C>,
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

    new_records_encryption_randomness: &[<C::AccountEncryption as EncryptionScheme>::Randomness],
    new_records_encryption_gadget_components: &[RecordEncryptionGadgetComponents<C>],
    new_encrypted_record_hashes: &[<C::EncryptedRecordCRH as CRH>::Output],

    // Rest
    program_commitment: &<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output,
    program_randomness: &<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness,
    local_data_root: &<C::LocalDataCRH as CRH>::Output,
    local_data_commitment_randomizers: &[<C::LocalDataCommitment as CommitmentScheme>::Randomness],
    memo: &[u8; 32],
    value_balance: i64,
    network_id: u8,
) -> Result<(), SynthesisError> {
    base_dpc_execute_gadget_helper::<
        C,
        CS,
        C::AccountCommitment,
        C::AccountEncryption,
        C::AccountSignature,
        C::RecordCommitment,
        C::EncryptedRecordCRH,
        C::LocalDataCRH,
        C::LocalDataCommitment,
        C::SerialNumberNonceCRH,
        C::PRF,
        C::AccountCommitmentGadget,
        C::AccountEncryptionGadget,
        C::AccountSignatureGadget,
        C::RecordCommitmentGadget,
        C::EncryptedRecordCRHGadget,
        C::LocalDataCRHGadget,
        C::LocalDataCommitmentGadget,
        C::SerialNumberNonceCRHGadget,
        C::PRFGadget,
    >(
        cs,
        //
        system_parameters,
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
        new_records_encryption_randomness,
        new_records_encryption_gadget_components,
        new_encrypted_record_hashes,
        //
        program_commitment,
        program_randomness,
        local_data_root,
        local_data_commitment_randomizers,
        memo,
        value_balance,
        network_id,
    )
}

fn base_dpc_execute_gadget_helper<
    C,
    CS: ConstraintSystem<C::InnerField>,
    AccountCommitment,
    AccountEncryption,
    AccountSignature,
    RecordCommitment,
    EncryptedRecordCRH,
    LocalDataCRH,
    LocalDataCommitment,
    SerialNumberNonceCRH,
    P,
    AccountCommitmentGadget,
    AccountEncryptionGadget,
    AccountSignatureGadget,
    RecordCommitmentGadget,
    EncryptedRecordCRHGadget,
    LocalDataCRHGadget,
    LocalDataCommitmentGadget,
    SerialNumberNonceCRHGadget,
    PGadget,
>(
    cs: &mut CS,

    //
    system_parameters: &SystemParameters<C>,
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

    new_records_encryption_randomness: &[<C::AccountEncryption as EncryptionScheme>::Randomness],
    new_records_encryption_gadget_components: &[RecordEncryptionGadgetComponents<C>],
    new_encrypted_record_hashes: &[EncryptedRecordCRH::Output],

    //
    program_commitment: &<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output,
    program_randomness: &<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness,
    local_data_root: &LocalDataCRH::Output,
    local_data_commitment_randomizers: &[LocalDataCommitment::Randomness],
    memo: &[u8; 32],
    value_balance: i64,
    network_id: u8,
) -> Result<(), SynthesisError>
where
    C: BaseDPCComponents<
        AccountCommitment = AccountCommitment,
        AccountEncryption = AccountEncryption,
        AccountSignature = AccountSignature,
        RecordCommitment = RecordCommitment,
        EncryptedRecordCRH = EncryptedRecordCRH,
        LocalDataCRH = LocalDataCRH,
        LocalDataCommitment = LocalDataCommitment,
        SerialNumberNonceCRH = SerialNumberNonceCRH,
        PRF = P,
        AccountCommitmentGadget = AccountCommitmentGadget,
        AccountEncryptionGadget = AccountEncryptionGadget,
        AccountSignatureGadget = AccountSignatureGadget,
        RecordCommitmentGadget = RecordCommitmentGadget,
        EncryptedRecordCRHGadget = EncryptedRecordCRHGadget,
        LocalDataCRHGadget = LocalDataCRHGadget,
        LocalDataCommitmentGadget = LocalDataCommitmentGadget,
        SerialNumberNonceCRHGadget = SerialNumberNonceCRHGadget,
        PRFGadget = PGadget,
    >,
    AccountCommitment: CommitmentScheme,
    AccountEncryption: EncryptionScheme,
    AccountSignature: SignatureScheme,
    RecordCommitment: CommitmentScheme,
    EncryptedRecordCRH: CRH,
    LocalDataCRH: CRH,
    LocalDataCommitment: CommitmentScheme,
    SerialNumberNonceCRH: CRH,
    P: PRF,
    RecordCommitment::Output: Eq,
    AccountCommitmentGadget: CommitmentGadget<AccountCommitment, C::InnerField>,
    AccountEncryptionGadget: EncryptionGadget<AccountEncryption, C::InnerField>,
    AccountSignatureGadget: SignaturePublicKeyRandomizationGadget<AccountSignature, C::InnerField>,
    RecordCommitmentGadget: CommitmentGadget<RecordCommitment, C::InnerField>,
    EncryptedRecordCRHGadget: CRHGadget<EncryptedRecordCRH, C::InnerField>,
    LocalDataCRHGadget: CRHGadget<LocalDataCRH, C::InnerField>,
    LocalDataCommitmentGadget: CommitmentGadget<LocalDataCommitment, C::InnerField>,
    SerialNumberNonceCRHGadget: CRHGadget<SerialNumberNonceCRH, C::InnerField>,
    PGadget: PRFGadget<P, C::InnerField>,
{
    let mut old_serial_numbers_gadgets = Vec::with_capacity(old_records.len());
    let mut old_serial_numbers_bytes_gadgets = Vec::with_capacity(old_records.len() * 32); // Serial numbers are 32 bytes
    let mut old_record_commitments_gadgets = Vec::with_capacity(old_records.len());
    let mut old_death_program_ids_gadgets = Vec::with_capacity(old_records.len());

    let mut new_record_commitments_gadgets = Vec::with_capacity(new_records.len());
    let mut new_birth_program_ids_gadgets = Vec::with_capacity(new_records.len());

    // Order for allocation of input:
    // 1. account_commitment_parameters
    // 2. account_encryption_parameters
    // 3. account_signature_parameters
    // 4. record_commitment_parameters
    // 5. encrypted_record_crh_parameters
    // 6. program_vk_commitment_parameters
    // 7. local_data_crh_parameters
    // 8. local_data_commitment_parameters
    // 9. serial_number_nonce_crh_parameters
    // 10. ledger_parameters
    // 11. ledger_digest
    // 12. for i in 0..NUM_INPUT_RECORDS: old_serial_numbers[i]
    // 13. for j in 0..NUM_OUTPUT_RECORDS: new_commitments[i], new_encrypted_record_hashes[i]
    // 14. program_commitment
    // 15. local_data_root
    let (
        account_commitment_parameters,
        account_encryption_parameters,
        account_signature_parameters,
        record_commitment_parameters,
        encrypted_record_crh_parameters,
        program_vk_commitment_parameters,
        local_data_crh_parameters,
        local_data_commitment_parameters,
        serial_number_nonce_crh_parameters,
        ledger_parameters,
    ) = {
        let cs = &mut cs.ns(|| "Declare commitment and CRH parameters");

        let account_commitment_parameters = AccountCommitmentGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare account commit parameters"),
            || Ok(system_parameters.account_commitment.parameters()),
        )?;

        let account_encryption_parameters = AccountEncryptionGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare account encryption parameters"),
            || Ok(system_parameters.account_encryption.parameters()),
        )?;

        let account_signature_parameters = AccountSignatureGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare account signature parameters"),
            || Ok(system_parameters.account_signature.parameters()),
        )?;

        let record_commitment_parameters = RecordCommitmentGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare record commitment parameters"),
            || Ok(system_parameters.record_commitment.parameters()),
        )?;

        let encrypted_record_crh_parameters = EncryptedRecordCRHGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare record ciphertext CRH parameters"),
            || Ok(system_parameters.encrypted_record_crh.parameters()),
        )?;

        let program_vk_commitment_parameters = <C::ProgramVerificationKeyCommitmentGadget as CommitmentGadget<
            _,
            C::InnerField,
        >>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare program vk commitment parameters"),
            || Ok(system_parameters.program_verification_key_commitment.parameters()),
        )?;

        let local_data_crh_parameters = LocalDataCRHGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare local data CRH parameters"),
            || Ok(system_parameters.local_data_crh.parameters()),
        )?;

        let local_data_commitment_parameters = LocalDataCommitmentGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare local data commitment parameters"),
            || Ok(system_parameters.local_data_commitment.parameters()),
        )?;

        let serial_number_nonce_crh_parameters = SerialNumberNonceCRHGadget::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare serial number nonce CRH parameters"),
            || Ok(system_parameters.serial_number_nonce.parameters()),
        )?;

        let ledger_parameters = <C::MerkleHashGadget as CRHGadget<_, _>>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare ledger parameters"),
            || Ok(ledger_parameters.parameters()),
        )?;

        (
            account_commitment_parameters,
            account_encryption_parameters,
            account_signature_parameters,
            record_commitment_parameters,
            encrypted_record_crh_parameters,
            program_vk_commitment_parameters,
            local_data_crh_parameters,
            local_data_commitment_parameters,
            serial_number_nonce_crh_parameters,
            ledger_parameters,
        )
    };

    let zero_value = UInt8::alloc_vec(&mut cs.ns(|| "Declare record zero value"), &to_bytes![0u64]?)?;

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
            given_record_owner,
            given_commitment,
            given_is_dummy,
            given_value,
            given_payload,
            given_birth_program_id,
            given_death_program_id,
            given_commitment_randomness,
            serial_number_nonce,
        ) = {
            let declare_cs = &mut cs.ns(|| "Declare input record");

            // No need to check that commitments, public keys and hashes are in
            // prime order subgroup because the commitment and CRH parameters
            // are trusted, and so when we recompute these, the newly computed
            // values will always be in correct subgroup. If the input cm, pk
            // or hash is incorrect, then it will not match the computed equivalent.
            let given_record_owner =
                AccountEncryptionGadget::PublicKeyGadget::alloc(&mut declare_cs.ns(|| "given_record_owner"), || {
                    Ok(record.owner().into_repr())
                })?;

            let given_commitment =
                RecordCommitmentGadget::OutputGadget::alloc(&mut declare_cs.ns(|| "given_commitment"), || {
                    Ok(record.commitment().clone())
                })?;
            old_record_commitments_gadgets.push(given_commitment.clone());

            let given_is_dummy = Boolean::alloc(&mut declare_cs.ns(|| "given_is_dummy"), || Ok(record.is_dummy()))?;

            let given_value = UInt8::alloc_vec(&mut declare_cs.ns(|| "given_value"), &to_bytes![record.value()]?)?;

            let given_payload = UInt8::alloc_vec(&mut declare_cs.ns(|| "given_payload"), &record.payload().to_bytes())?;

            let given_birth_program_id = UInt8::alloc_vec(
                &mut declare_cs.ns(|| "given_birth_program_id"),
                &record.birth_program_id(),
            )?;

            let given_death_program_id = UInt8::alloc_vec(
                &mut declare_cs.ns(|| "given_death_program_id"),
                &record.death_program_id(),
            )?;
            old_death_program_ids_gadgets.push(given_death_program_id.clone());

            let given_commitment_randomness = RecordCommitmentGadget::RandomnessGadget::alloc(
                &mut declare_cs.ns(|| "given_commitment_randomness"),
                || Ok(record.commitment_randomness()),
            )?;

            let serial_number_nonce =
                SerialNumberNonceCRHGadget::OutputGadget::alloc(&mut declare_cs.ns(|| "serial_number_nonce"), || {
                    Ok(record.serial_number_nonce())
                })?;
            (
                given_record_owner,
                given_commitment,
                given_is_dummy,
                given_value,
                given_payload,
                given_birth_program_id,
                given_death_program_id,
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
        // Check that the account address and private key form a valid key
        // pair.
        // ********************************************************************

        let (sk_prf, pk_sig) = {
            // Declare variables for account contents.
            let account_cs = &mut cs.ns(|| "Check account");

            // Allocate the account private key.
            let (pk_sig, sk_prf, r_pk) = {
                let pk_sig_native = account_private_key
                    .pk_sig(&system_parameters.account_signature)
                    .map_err(|_| SynthesisError::AssignmentMissing)?;
                let pk_sig =
                    AccountSignatureGadget::PublicKeyGadget::alloc(&mut account_cs.ns(|| "Declare pk_sig"), || {
                        Ok(&pk_sig_native)
                    })?;
                let sk_prf = PGadget::new_seed(&mut account_cs.ns(|| "Declare sk_prf"), &account_private_key.sk_prf);
                let r_pk =
                    AccountCommitmentGadget::RandomnessGadget::alloc(&mut account_cs.ns(|| "Declare r_pk"), || {
                        Ok(&account_private_key.r_pk)
                    })?;

                (pk_sig, sk_prf, r_pk)
            };

            // Construct the account view key.
            let candidate_account_view_key = {
                let mut account_view_key_input = pk_sig.to_bytes(&mut account_cs.ns(|| "pk_sig to_bytes"))?;
                account_view_key_input.extend_from_slice(&sk_prf);

                // This is the record decryption key.
                let candidate_account_commitment = AccountCommitmentGadget::check_commitment_gadget(
                    &mut account_cs.ns(|| "Compute the account commitment."),
                    &account_commitment_parameters,
                    &account_view_key_input,
                    &r_pk,
                )?;

                // TODO (howardwu): Enforce 6 MSB bits are 0.
                {
                    // TODO (howardwu): Enforce 6 MSB bits are 0.
                }

                // Enforce the account commitment bytes (padded) correspond to the
                // given account's view key bytes (padded). This is equivalent to
                // verifying that the base field element from the computed account
                // commitment contains the same bit-value as the scalar field element
                // computed from the given account private key.
                let given_account_view_key = {
                    // Derive the given account view key based on the given account private key.
                    let given_account_view_key = AccountEncryptionGadget::PrivateKeyGadget::alloc(
                        &mut account_cs.ns(|| "Allocate account view key"),
                        || {
                            Ok(account_private_key
                                .to_decryption_key(
                                    &system_parameters.account_signature,
                                    &system_parameters.account_commitment,
                                )
                                .map_err(|_| SynthesisError::AssignmentMissing)?)
                        },
                    )?;

                    let given_account_view_key_bytes =
                        given_account_view_key.to_bytes(&mut account_cs.ns(|| "given_account_view_key to_bytes"))?;

                    let candidate_account_commitment_bytes = candidate_account_commitment
                        .to_bytes(&mut account_cs.ns(|| "candidate_account_commitment to_bytes"))?;

                    candidate_account_commitment_bytes.enforce_equal(
                        &mut account_cs.ns(|| "Check that candidate and given account view keys are equal"),
                        &given_account_view_key_bytes,
                    )?;

                    given_account_view_key
                };

                given_account_view_key
            };

            // Construct and verify the record owner - account address.
            {
                let candidate_record_owner = AccountEncryptionGadget::check_public_key_gadget(
                    &mut account_cs.ns(|| "Compute the candidate record owner - account address"),
                    &account_encryption_parameters,
                    &candidate_account_view_key,
                )?;

                candidate_record_owner.enforce_equal(
                    &mut account_cs.ns(|| "Check that declared and computed addresses are equal"),
                    &given_record_owner,
                )?;
            }

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

        // *******************************************************************
        // Check that the record is well-formed.
        // *******************************************************************
        {
            let commitment_cs = &mut cs.ns(|| "Check that record is well-formed");

            given_value.conditional_enforce_equal(
                &mut commitment_cs
                    .ns(|| format!("Enforce that if old record {} is a dummy, that it has a value of 0", i)),
                &zero_value,
                &given_is_dummy,
            )?;

            let record_owner_bytes =
                given_record_owner.to_bytes(&mut commitment_cs.ns(|| "Convert record_owner to bytes"))?;
            let is_dummy_bytes = given_is_dummy.to_bytes(&mut commitment_cs.ns(|| "Convert is_dummy to bytes"))?;

            let mut commitment_input = Vec::new();
            commitment_input.extend_from_slice(&record_owner_bytes);
            commitment_input.extend_from_slice(&is_dummy_bytes);
            commitment_input.extend_from_slice(&given_value);
            commitment_input.extend_from_slice(&given_payload);
            commitment_input.extend_from_slice(&given_birth_program_id);
            commitment_input.extend_from_slice(&given_death_program_id);
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

    for (
        j,
        (
            ((((record, sn_nonce_randomness), commitment), encryption_randomness), encryption_gadget_components),
            encrypted_record_hash,
        ),
    ) in new_records
        .iter()
        .zip(new_sn_nonce_randomness)
        .zip(new_commitments)
        .zip(new_records_encryption_randomness)
        .zip(new_records_encryption_gadget_components)
        .zip(new_encrypted_record_hashes)
        .enumerate()
    {
        let RecordEncryptionGadgetComponents {
            record_field_elements,
            record_group_encoding,
            ciphertext_selectors,
            fq_high_selectors,
            encryption_blinding_exponents,
        } = encryption_gadget_components;

        let cs = &mut cs.ns(|| format!("Process output record {}", j));

        let (
            given_record_owner,
            given_record_commitment,
            given_commitment,
            given_is_dummy,
            given_value,
            given_payload,
            given_birth_program_id,
            given_death_program_id,
            given_commitment_randomness,
            serial_number_nonce,
            serial_number_nonce_bytes,
        ) = {
            let declare_cs = &mut cs.ns(|| "Declare output record");

            let given_record_owner =
                AccountEncryptionGadget::PublicKeyGadget::alloc(&mut declare_cs.ns(|| "given_record_owner"), || {
                    Ok(record.owner().into_repr())
                })?;

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

            let given_value = UInt8::alloc_vec(&mut declare_cs.ns(|| "given_value"), &to_bytes![record.value()]?)?;

            let given_payload = UInt8::alloc_vec(&mut declare_cs.ns(|| "given_payload"), &record.payload().to_bytes())?;

            let given_birth_program_id = UInt8::alloc_vec(
                &mut declare_cs.ns(|| "given_birth_program_id"),
                &record.birth_program_id(),
            )?;
            new_birth_program_ids_gadgets.push(given_birth_program_id.clone());

            let given_death_program_id = UInt8::alloc_vec(
                &mut declare_cs.ns(|| "given_death_program_id"),
                &record.death_program_id(),
            )?;

            let given_commitment_randomness = RecordCommitmentGadget::RandomnessGadget::alloc(
                &mut declare_cs.ns(|| "given_commitment_randomness"),
                || Ok(record.commitment_randomness()),
            )?;

            let serial_number_nonce =
                SerialNumberNonceCRHGadget::OutputGadget::alloc(&mut declare_cs.ns(|| "serial_number_nonce"), || {
                    Ok(record.serial_number_nonce())
                })?;

            let serial_number_nonce_bytes =
                serial_number_nonce.to_bytes(&mut declare_cs.ns(|| "Convert sn nonce to bytes"))?;

            (
                given_record_owner,
                given_record_commitment,
                given_commitment,
                given_is_dummy,
                given_value,
                given_payload,
                given_birth_program_id,
                given_death_program_id,
                given_commitment_randomness,
                serial_number_nonce,
                serial_number_nonce_bytes,
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
        // *******************************************************************

        // *******************************************************************
        // Check that the record is well-formed.
        // *******************************************************************
        {
            let commitment_cs = &mut cs.ns(|| "Check that record is well-formed");

            given_value.conditional_enforce_equal(
                &mut commitment_cs
                    .ns(|| format!("Enforce that if new record {} is a dummy, that it has a value of 0", j)),
                &zero_value,
                &given_is_dummy,
            )?;

            let record_owner_bytes =
                given_record_owner.to_bytes(&mut commitment_cs.ns(|| "Convert record_owner to bytes"))?;
            let is_dummy_bytes = given_is_dummy.to_bytes(&mut commitment_cs.ns(|| "Convert is_dummy to bytes"))?;

            let mut commitment_input = Vec::new();
            commitment_input.extend_from_slice(&record_owner_bytes);
            commitment_input.extend_from_slice(&is_dummy_bytes);
            commitment_input.extend_from_slice(&given_value);
            commitment_input.extend_from_slice(&given_payload);
            commitment_input.extend_from_slice(&given_birth_program_id);
            commitment_input.extend_from_slice(&given_death_program_id);
            commitment_input.extend_from_slice(&serial_number_nonce_bytes);

            let candidate_commitment = RecordCommitmentGadget::check_commitment_gadget(
                &mut commitment_cs.ns(|| "Compute record commitment"),
                &record_commitment_parameters,
                &commitment_input,
                &given_commitment_randomness,
            )?;
            candidate_commitment.enforce_equal(
                &mut commitment_cs.ns(|| "Check that computed commitment matches public input"),
                &given_commitment,
            )?;
            candidate_commitment.enforce_equal(
                &mut commitment_cs.ns(|| "Check that computed commitment matches declared commitment"),
                &given_record_commitment,
            )?;
        }

        // *******************************************************************

        // *******************************************************************
        // Check that the record encryption is well-formed.
        // *******************************************************************
        {
            let encryption_cs = &mut cs.ns(|| "Check that record encryption is well-formed");

            // Check serialization

            // *******************************************************************
            // Convert serial number nonce, commitment_randomness, birth program id, death program id, payload, and value into bits

            let serial_number_nonce_bits = serial_number_nonce_bytes
                .to_bits(&mut encryption_cs.ns(|| "Convert serial_number_nonce_bytes to bits"))?;

            let commitment_randomness_bytes =
                UInt8::alloc_vec(encryption_cs.ns(|| "Allocate commitment randomness bytes"), &to_bytes![
                    record.commitment_randomness()
                ]?)?;

            let commitment_randomness_bits = commitment_randomness_bytes
                .to_bits(&mut encryption_cs.ns(|| "Convert commitment_randomness_bytes to bits"))?;
            let full_birth_program_id_bits =
                given_birth_program_id.to_bits(&mut encryption_cs.ns(|| "Convert given_birth_program_id to bits"))?;
            let full_death_program_id_bits =
                given_death_program_id.to_bits(&mut encryption_cs.ns(|| "Convert given_death_program_id to bits"))?;
            let value_bits = given_value.to_bits(&mut encryption_cs.ns(|| "Convert given_value to bits"))?;
            let payload_bits = given_payload.to_bits(&mut encryption_cs.ns(|| "Convert given_payload to bits"))?;
            let mut fq_high_bits = vec![];

            for (i, fq_high_bit) in fq_high_selectors[0..(fq_high_selectors.len() - 1)].iter().enumerate() {
                let boolean = Boolean::alloc(
                    encryption_cs.ns(|| format!("Allocate fq_high_bit {} - {}", i, j)),
                    || Ok(fq_high_bit),
                )?;
                fq_high_bits.push(boolean);
            }

            // *******************************************************************
            // Pack the record bits into serialization format

            let scalar_field_bitsize = <C::EncryptionGroup as Group>::ScalarField::size_in_bits();
            let base_field_bitsize = <C::InnerField as PrimeField>::size_in_bits();
            let outer_field_bitsize = <C::OuterField as PrimeField>::size_in_bits();

            // A standard unit for packing bits into data storage
            let data_field_bitsize = base_field_bitsize - 1;

            // Assumption 1 - The scalar field bit size must be strictly less than the base field bit size
            // for the logic below to work correctly.
            assert!(scalar_field_bitsize < base_field_bitsize);

            // Assumption 2 - this implementation assumes the outer field bit size is larger than
            // the data field bit size by at most one additional scalar field bit size.
            assert!((outer_field_bitsize - data_field_bitsize) <= data_field_bitsize);

            // Assumption 3 - this implementation assumes the remainder of two outer field bit sizes
            // can fit within one data field element's bit size.
            assert!((2 * (outer_field_bitsize - data_field_bitsize)) <= data_field_bitsize);

            // Assumption 4 - this implementation assumes the payload and value may be zero values.
            // As such, to ensure the values are non-zero for encoding and decoding, we explicitly
            // reserve the MSB of the data field element's valid bitsize and set the bit to 1.
            let payload_field_bitsize = data_field_bitsize - 1;

            // Birth and death programs

            let mut birth_program_id_bits = Vec::with_capacity(base_field_bitsize);
            let mut death_program_id_bits = Vec::with_capacity(base_field_bitsize);
            let mut birth_program_id_remainder_bits = Vec::with_capacity(outer_field_bitsize - data_field_bitsize);
            let mut death_program_id_remainder_bits = Vec::with_capacity(outer_field_bitsize - data_field_bitsize);

            for i in 0..data_field_bitsize {
                birth_program_id_bits.push(full_birth_program_id_bits[i]);
                death_program_id_bits.push(full_death_program_id_bits[i]);
            }

            // (Assumption 2 applies)
            for i in data_field_bitsize..outer_field_bitsize {
                birth_program_id_remainder_bits.push(full_birth_program_id_bits[i]);
                death_program_id_remainder_bits.push(full_death_program_id_bits[i]);
            }
            birth_program_id_remainder_bits.extend_from_slice(&death_program_id_remainder_bits);

            // Payload

            let mut payload_elements = vec![];

            let mut payload_field_bits = Vec::with_capacity(payload_field_bitsize + 1);

            for (i, bit) in payload_bits.iter().enumerate() {
                payload_field_bits.push(*bit);

                if (i > 0) && ((i + 1) % payload_field_bitsize == 0) {
                    // (Assumption 4)
                    payload_field_bits.push(Boolean::Constant(true));

                    payload_elements.push(payload_field_bits.clone());

                    payload_field_bits.clear();
                }
            }

            let num_payload_elements = payload_bits.len() / payload_field_bitsize;
            assert_eq!(payload_elements.len(), num_payload_elements);

            // Determine if value can fit in current payload_field_bits.

            let value_does_not_fit =
                (payload_field_bits.len() + fq_high_bits.len() + value_bits.len()) > payload_field_bitsize;

            if value_does_not_fit {
                // (Assumption 4)
                payload_field_bits.push(Boolean::Constant(true));

                payload_elements.push(payload_field_bits.clone());

                payload_field_bits.clear();
            }

            assert_eq!(
                payload_elements.len(),
                num_payload_elements + (value_does_not_fit as usize)
            );

            let fq_high_and_payload_and_value_bits = [
                &vec![Boolean::Constant(true)],
                &fq_high_bits[..],
                &value_bits[..],
                &payload_field_bits[..],
            ]
            .concat();
            payload_elements.push(fq_high_and_payload_and_value_bits.clone());

            let num_payload_elements = payload_bits.len() / payload_field_bitsize;

            assert_eq!(
                payload_elements.len(),
                num_payload_elements + (value_does_not_fit as usize) + 1
            );

            // *******************************************************************
            // Alloc each of the record field elements as gadgets.

            let mut record_field_elements_gadgets = Vec::with_capacity(record_field_elements.len());

            for (i, element) in record_field_elements.iter().enumerate() {
                let record_field_element_gadget =
                    Elligator2FieldGadget::<C::EncryptionModelParameters, C::InnerField>::alloc(
                        &mut encryption_cs.ns(|| format!("record_field_element_{}", i)),
                        || Ok(*element),
                    )?;

                record_field_elements_gadgets.push(record_field_element_gadget);
            }

            // *******************************************************************
            // Feed in Field elements of the serialization and convert them to bits

            let given_serial_number_nonce_bytes = &record_field_elements_gadgets[0]
                .to_bytes(&mut encryption_cs.ns(|| "given_serial_number_nonce_bytes"))?;
            let given_serial_number_nonce_bits = given_serial_number_nonce_bytes
                .to_bits(&mut encryption_cs.ns(|| "Convert given_serial_number_nonce_bytes to bits"))?;

            let given_commitment_randomness_bytes = &record_field_elements_gadgets[1]
                .to_bytes(&mut encryption_cs.ns(|| "given_commitment_randomness_bytes"))?;
            let given_commitment_randomness_bits = given_commitment_randomness_bytes
                .to_bits(&mut encryption_cs.ns(|| "Convert given_commitment_randomness_bytes to bits"))?;

            let given_birth_program_id_bytes =
                &record_field_elements_gadgets[2].to_bytes(&mut encryption_cs.ns(|| "given_birth_program_id_bytes"))?;
            let given_birth_program_id_bits = given_birth_program_id_bytes
                .to_bits(&mut encryption_cs.ns(|| "Convert given_birth_program_id_bytes to bits"))?;

            let given_death_program_id_bytes =
                &record_field_elements_gadgets[3].to_bytes(&mut encryption_cs.ns(|| "given_death_program_id_bytes"))?;
            let given_death_program_id_bits = given_death_program_id_bytes
                .to_bits(&mut encryption_cs.ns(|| "Convert given_death_program_id_bytes to bits"))?;

            let given_program_id_remainder_bytes = &record_field_elements_gadgets[4]
                .to_bytes(&mut encryption_cs.ns(|| "given_program_id_remainder_bytes"))?;
            let given_program_id_remainder_bits = given_program_id_remainder_bytes
                .to_bits(&mut encryption_cs.ns(|| "Convert given_program_id_remainder_bytes to bits"))?;

            // *******************************************************************
            // Equate the gadget packed and provided bits

            serial_number_nonce_bits.enforce_equal(
                &mut encryption_cs.ns(|| "Check that computed and declared serial_number_nonce_bits match"),
                &given_serial_number_nonce_bits,
            )?;

            commitment_randomness_bits.enforce_equal(
                &mut encryption_cs.ns(|| "Check that computed and declared commitment_randomness_bits match"),
                &given_commitment_randomness_bits,
            )?;

            birth_program_id_bits.enforce_equal(
                &mut encryption_cs.ns(|| "Check that computed and declared given_birth_program_id_bits match"),
                &given_birth_program_id_bits,
            )?;

            death_program_id_bits.enforce_equal(
                &mut encryption_cs.ns(|| "Check that computed and declared death_program_id_bits match"),
                &given_death_program_id_bits,
            )?;

            birth_program_id_remainder_bits.enforce_equal(
                &mut encryption_cs.ns(|| "Check that computed and declared program_id_remainder_bits match"),
                &given_program_id_remainder_bits,
            )?;

            for (i, (payload_element, field_element)) in payload_elements
                .iter()
                .zip(&record_field_elements_gadgets[5..])
                .enumerate()
            {
                let given_element_bytes =
                    field_element.to_bytes(&mut encryption_cs.ns(|| format!("given_payload_bytes - {}", i)))?;
                let given_element_bits = given_element_bytes
                    .to_bits(&mut encryption_cs.ns(|| format!("Convert given_payload_bytes - {} to bits", i)))?;

                payload_element.enforce_equal(
                    &mut encryption_cs.ns(|| format!("Check that computed and declared payload_bits match {}", i)),
                    &given_element_bits,
                )?;
            }

            // *******************************************************************
            // Check group encoding correctness

            let mut record_group_encoding_gadgets = Vec::with_capacity(record_group_encoding.len());
            let mut encryption_plaintext = Vec::with_capacity(record_group_encoding.len());

            for (i, (x, y)) in record_group_encoding.iter().enumerate() {
                let affine = <C::EncryptionGroup as ProjectiveCurve>::Affine::read(&to_bytes![x, y]?[..])?;
                encryption_plaintext.push(<C::AccountEncryption as EncryptionScheme>::Text::read(
                    &to_bytes![affine.into_projective()]?[..],
                )?);

                let y_gadget = Elligator2FieldGadget::<C::EncryptionModelParameters, C::InnerField>::alloc(
                    &mut encryption_cs.ns(|| format!("record_group_encoding_y_{}", i)),
                    || Ok(y),
                )?;

                record_group_encoding_gadgets.push(y_gadget);
            }

            assert_eq!(record_field_elements_gadgets.len(), record_group_encoding_gadgets.len());

            let coeff_a = <C::EncryptionModelParameters as MontgomeryModelParameters>::COEFF_A;
            let coeff_b = <C::EncryptionModelParameters as MontgomeryModelParameters>::COEFF_B;

            let a = coeff_a.mul(&coeff_b.inverse().unwrap());
            let u = <C::EncryptionModelParameters as TEModelParameters>::COEFF_D;
            let ua = a.mul(&u);

            let a = C::InnerField::read(&to_bytes![a]?[..])?;
            let b = C::InnerField::read(&to_bytes![coeff_b]?[..])?;
            let u = C::InnerField::read(&to_bytes![u]?[..])?;
            let ua = C::InnerField::read(&to_bytes![ua]?[..])?;

            for (i, (element, y_gadget)) in record_field_elements_gadgets
                .iter()
                .skip(1)
                .zip(record_group_encoding_gadgets.iter().skip(1))
                .enumerate()
            {
                // Get reconstructed x value
                let numerator = y_gadget
                    .0
                    .add_constant(encryption_cs.ns(|| format!("1 + y_{}", i)), &C::InnerField::one())?;
                let neg_y = y_gadget.0.negate(encryption_cs.ns(|| format!("-y_{}", i)))?;
                let denominator = neg_y
                    .add_constant(encryption_cs.ns(|| format!("1 - y_{}", i)), &C::InnerField::one())?
                    .inverse(encryption_cs.ns(|| format!("(1 - y_{})_inverse", i)))?;

                let temp_u = numerator.mul(
                    encryption_cs.ns(|| format!("u = (1 + y_{}) * (1 - y_{})_inverse", i, i)),
                    &denominator,
                )?;
                let x = temp_u.mul_by_constant(
                    encryption_cs.ns(|| format!("(1 + y_{}) * (1 - y_{})_inverse * b_inverse", i, i)),
                    &b.inverse().unwrap(),
                )?;

                let ux = x.mul_by_constant(encryption_cs.ns(|| format!("x_{} * u", i)), &u)?;
                let neg_x = x.negate(encryption_cs.ns(|| format!("-x_{}", i)))?;

                // Construct a_i
                let ux_plus_ua = ux.add_constant(encryption_cs.ns(|| format!("ux_{} + uA", i)), &ua)?;
                let ux_plus_ua_inverse = ux_plus_ua.inverse(encryption_cs.ns(|| format!("1 d (ux_{} + uA)", i)))?;
                let a_i = neg_x.mul(
                    encryption_cs.ns(|| format!("-x_{} * (ux + uA)_inverse", i)),
                    &ux_plus_ua_inverse,
                )?;

                // Construct b_i
                let neg_x_minus_a = neg_x.add_constant(encryption_cs.ns(|| format!("-x_{} - A", i)), &-a)?;
                let ux_inverse = ux.inverse(encryption_cs.ns(|| format!("ux_{}_inverse", i)))?;
                let b_i =
                    neg_x_minus_a.mul(encryption_cs.ns(|| format!("(-x_{} - A) * ux_inverse", i)), &ux_inverse)?;

                let element_squared = element
                    .0
                    .mul(encryption_cs.ns(|| format!("element_{} ^ 2", i)), &element.0)?;

                let a_i_is_correct =
                    element_squared.evaluate_equal(encryption_cs.ns(|| format!("element_squared == a_{}", i)), &a_i)?;
                let b_i_is_correct =
                    element_squared.evaluate_equal(encryption_cs.ns(|| format!("element_squared == b_{}", i)), &b_i)?;

                // Enforce that either a_i or b_i was valid
                let single_valid_recovery = a_i_is_correct.evaluate_equal(
                    encryption_cs.ns(|| format!("(element_squared == a_{}) == (element_squared == b_{})", i, i)),
                    &b_i_is_correct,
                )?;
                single_valid_recovery.enforce_equal(
                    encryption_cs.ns(|| format!("single_valid_recovery_{} == false", i)),
                    &Boolean::Constant(false),
                )?;
            }

            // *******************************************************************
            // Construct the record encryption

            let encryption_randomness_gadget = AccountEncryptionGadget::RandomnessGadget::alloc(
                &mut encryption_cs.ns(|| format!("output record {} encryption_randomness", j)),
                || Ok(encryption_randomness),
            )?;

            let encryption_blinding_exponents_gadget = AccountEncryptionGadget::BlindingExponentGadget::alloc(
                &mut encryption_cs.ns(|| format!("output record {} encryption_blinding_exponents", j)),
                || Ok(encryption_blinding_exponents),
            )?;

            let encryption_plaintext_gadget = AccountEncryptionGadget::PlaintextGadget::alloc(
                &mut encryption_cs.ns(|| format!("output record {} encryption_plaintext", j)),
                || Ok(encryption_plaintext),
            )?;

            let candidate_encrypted_record_gadget = AccountEncryptionGadget::check_encryption_gadget(
                &mut encryption_cs.ns(|| format!("output record {} check_encryption_gadget", j)),
                &account_encryption_parameters,
                &encryption_randomness_gadget,
                &given_record_owner,
                &encryption_plaintext_gadget,
                &encryption_blinding_exponents_gadget,
            )?;

            // *******************************************************************
            // Check that the encrypted record hash is correct

            let encrypted_record_hash_gadget = EncryptedRecordCRHGadget::OutputGadget::alloc_input(
                &mut encryption_cs.ns(|| format!("output record {} encrypted record hash", j)),
                || Ok(encrypted_record_hash),
            )?;

            let candidate_encrypted_record_bytes = candidate_encrypted_record_gadget
                .to_bytes(encryption_cs.ns(|| format!("output record {} encrypted record bytes", j)))?;

            let ciphertext_and_fq_high_selectors_bytes = UInt8::alloc_vec(
                &mut encryption_cs.ns(|| format!("ciphertext and fq_high selector bits to bytes {}", j)),
                &bits_to_bytes(
                    &[&ciphertext_selectors[..], &[
                        fq_high_selectors[fq_high_selectors.len() - 1]
                    ]]
                    .concat(),
                ),
            )?;

            let mut encrypted_record_hash_input = Vec::new();
            encrypted_record_hash_input.extend_from_slice(&candidate_encrypted_record_bytes);
            encrypted_record_hash_input.extend_from_slice(&ciphertext_and_fq_high_selectors_bytes);

            let candidate_encrypted_record_hash = EncryptedRecordCRHGadget::check_evaluation_gadget(
                &mut encryption_cs.ns(|| format!("Compute encrypted record hash {}", j)),
                &encrypted_record_crh_parameters,
                &encrypted_record_hash_input,
            )?;

            encrypted_record_hash_gadget.enforce_equal(
                encryption_cs.ns(|| format!("output record {} encrypted record hash is valid", j)),
                &candidate_encrypted_record_hash,
            )?;
        }
    }
    // *******************************************************************

    // *******************************************************************
    // Check that program commitment is well formed.
    // *******************************************************************
    {
        let commitment_cs = &mut cs.ns(|| "Check that program commitment is well-formed");

        let mut input = Vec::new();
        for i in 0..C::NUM_INPUT_RECORDS {
            input.extend_from_slice(&old_death_program_ids_gadgets[i]);
        }

        for j in 0..C::NUM_OUTPUT_RECORDS {
            input.extend_from_slice(&new_birth_program_ids_gadgets[j]);
        }

        let given_commitment_randomness =
            <C::ProgramVerificationKeyCommitmentGadget as CommitmentGadget<_, C::InnerField>>::RandomnessGadget::alloc(
                &mut commitment_cs.ns(|| "given_commitment_randomness"),
                || Ok(program_randomness),
            )?;

        let given_commitment = <C::ProgramVerificationKeyCommitmentGadget as CommitmentGadget<_, C::InnerField>>::OutputGadget::alloc_input(
            &mut commitment_cs.ns(|| "given_commitment"),
            || Ok(program_commitment),
        )?;

        let candidate_commitment =
            <C::ProgramVerificationKeyCommitmentGadget as CommitmentGadget<_, C::InnerField>>::check_commitment_gadget(
                &mut commitment_cs.ns(|| "candidate_commitment"),
                &program_vk_commitment_parameters,
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
    // Check that the local data root is valid
    // ********************************************************************
    {
        let mut cs = cs.ns(|| "Check that local data root is valid.");

        let memo = UInt8::alloc_input_vec(cs.ns(|| "Allocate memorandum"), memo)?;
        let network_id = UInt8::alloc_input_vec(cs.ns(|| "Allocate network id"), &[network_id])?;

        let mut old_record_commitment_bytes = vec![];
        for i in 0..C::NUM_INPUT_RECORDS {
            let mut cs = cs.ns(|| format!("Construct local data with input record {}", i));

            let mut input_bytes = vec![];
            input_bytes.extend_from_slice(&old_serial_numbers_gadgets[i].to_bytes(&mut cs.ns(|| "old_serial_number"))?);
            input_bytes.extend_from_slice(
                &old_record_commitments_gadgets[i].to_bytes(&mut cs.ns(|| "old_record_commitment"))?,
            );
            input_bytes.extend_from_slice(&memo);
            input_bytes.extend_from_slice(&network_id);

            let commitment_randomness = LocalDataCommitmentGadget::RandomnessGadget::alloc(
                cs.ns(|| format!("Allocate old record local data commitment randomness {}", i)),
                || Ok(&local_data_commitment_randomizers[i]),
            )?;

            let commitment = LocalDataCommitmentGadget::check_commitment_gadget(
                cs.ns(|| format!("Commit to old record local data {}", i)),
                &local_data_commitment_parameters,
                &input_bytes,
                &commitment_randomness,
            )?;

            old_record_commitment_bytes
                .extend_from_slice(&commitment.to_bytes(&mut cs.ns(|| "old_record_local_data"))?);
        }

        let mut new_record_commitment_bytes = Vec::new();
        for j in 0..C::NUM_OUTPUT_RECORDS {
            let mut cs = cs.ns(|| format!("Construct local data with output record {}", j));

            let mut input_bytes = vec![];
            input_bytes
                .extend_from_slice(&new_record_commitments_gadgets[j].to_bytes(&mut cs.ns(|| "record_commitment"))?);
            input_bytes.extend_from_slice(&memo);
            input_bytes.extend_from_slice(&network_id);

            let commitment_randomness = LocalDataCommitmentGadget::RandomnessGadget::alloc(
                cs.ns(|| format!("Allocate new record local data commitment randomness {}", j)),
                || Ok(&local_data_commitment_randomizers[C::NUM_INPUT_RECORDS + j]),
            )?;

            let commitment = LocalDataCommitmentGadget::check_commitment_gadget(
                cs.ns(|| format!("Commit to new record local data {}", j)),
                &local_data_commitment_parameters,
                &input_bytes,
                &commitment_randomness,
            )?;

            new_record_commitment_bytes
                .extend_from_slice(&commitment.to_bytes(&mut cs.ns(|| "new_record_local_data"))?);
        }

        let inner1_commitment_hash = LocalDataCRHGadget::check_evaluation_gadget(
            cs.ns(|| "Compute to local data commitment inner1 hash"),
            &local_data_crh_parameters,
            &old_record_commitment_bytes,
        )?;

        let inner2_commitment_hash = LocalDataCRHGadget::check_evaluation_gadget(
            cs.ns(|| "Compute to local data commitment inner2 hash"),
            &local_data_crh_parameters,
            &new_record_commitment_bytes,
        )?;

        let mut inner_commitment_hash_bytes = Vec::new();
        inner_commitment_hash_bytes
            .extend_from_slice(&inner1_commitment_hash.to_bytes(&mut cs.ns(|| "inner1_commitment_hash"))?);
        inner_commitment_hash_bytes
            .extend_from_slice(&inner2_commitment_hash.to_bytes(&mut cs.ns(|| "inner2_commitment_hash"))?);

        let candidate_local_data_root = LocalDataCRHGadget::check_evaluation_gadget(
            cs.ns(|| "Compute to local data commitment root"),
            &local_data_crh_parameters,
            &inner_commitment_hash_bytes,
        )?;

        let declared_local_data_root =
            LocalDataCRHGadget::OutputGadget::alloc_input(cs.ns(|| "Allocate local data root"), || {
                Ok(local_data_root)
            })?;

        candidate_local_data_root.enforce_equal(
            &mut cs.ns(|| "Check that local data root is valid"),
            &declared_local_data_root,
        )?;
    }
    // *******************************************************************

    // *******************************************************************
    // Check that the value balance is valid
    // *******************************************************************
    {
        let mut cs = cs.ns(|| "Check that the value balance is valid.");

        let given_value_balance = Int64::alloc_input_fe(cs.ns(|| "given_value_balance"), value_balance)?;

        let mut candidate_value_balance = Int64::zero();

        for (i, old_record) in old_records.iter().enumerate() {
            let value = old_record.value as i64;
            let record_value = Int64::alloc(cs.ns(|| format!("old record {} value", i)), || Ok(value))?;

            candidate_value_balance = candidate_value_balance
                .add(cs.ns(|| format!("add old record {} value", i)), &record_value)
                .unwrap();
        }

        for (j, new_record) in new_records.iter().enumerate() {
            let value = new_record.value as i64;
            let record_value = Int64::alloc(cs.ns(|| format!("new record {} value", j)), || Ok(value))?;

            candidate_value_balance = candidate_value_balance
                .sub(cs.ns(|| format!("sub new record {} value", j)), &record_value)
                .unwrap();
        }

        // Enforce that given_value_balance is equivalent to candidate_value_balance
        given_value_balance.enforce_equal(
            cs.ns(|| "given_value_balance == candidate_value_balance"),
            &given_value_balance,
        )?;
    }

    Ok(())
}
