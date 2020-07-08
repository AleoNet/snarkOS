use super::record_serializer::*;
use crate::dpc::base_dpc::{instantiated::*, record_payload::RecordPayload, DPC};
use snarkos_algorithms::crh::bytes_to_bits;
use snarkos_curves::edwards_bls12::EdwardsProjective as EdwardsBls;
use snarkos_models::{algorithms::CRH, objects::AccountScheme};

use snarkos_objects::Account;

use snarkos_utilities::{bytes::ToBytes, to_bytes};

//use rand::SeedableRng;
//use rand_xorshift::XorShiftRng;
use rand::{thread_rng, Rng};

#[test]
fn test_bits_to_bytes() {
    let mut rng = thread_rng();

    let bytes: [u8; 32] = rng.gen();

    let bits = bytes_to_bits(&bytes);

    let recovered_bytes = bits_to_bytes(&bits);
    assert_eq!(bytes.to_vec(), recovered_bytes);
}

#[test]
fn test_record_serialization() {
    //    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    let mut rng = thread_rng();

    // Generate parameters for the ledger, commitment schemes, CRH, and the
    // "always-accept" predicate.
    let circuit_parameters = InstantiatedDPC::generate_circuit_parameters(&mut rng).unwrap();
    let pred_nizk_pp = InstantiatedDPC::generate_predicate_snark_parameters(&circuit_parameters, &mut rng).unwrap();

    let pred_nizk_vk_bytes = to_bytes![
        PredicateVerificationKeyHash::hash(
            &circuit_parameters.predicate_verification_key_hash,
            &to_bytes![pred_nizk_pp.verification_key].unwrap()
        )
        .unwrap()
    ]
    .unwrap();

    let signature_parameters = &circuit_parameters.account_signature;
    let commitment_parameters = &circuit_parameters.account_commitment;

    // Generate metadata and an account for a dummy initial record.
    let meta_data = [1u8; 32];
    let dummy_account = Account::new(signature_parameters, commitment_parameters, &meta_data, &mut rng).unwrap();

    // Use genesis record, serial number, and memo to initialize the ledger.

    let sn_nonce = SerialNumberNonce::hash(&circuit_parameters.serial_number_nonce, &[0u8; 1]).unwrap();
    let record = DPC::generate_record(
        &circuit_parameters,
        &sn_nonce,
        &dummy_account.public_key,
        true,
        0,
        &RecordPayload::default(),
        &Predicate::new(pred_nizk_vk_bytes.clone()),
        &Predicate::new(pred_nizk_vk_bytes.clone()),
        &mut rng,
    )
    .unwrap();

    println!("birth pred repr: {:?}", record.birth_predicate_repr);

    let serialized_record = RecordSerializer::<_, EdwardsBls>::serialize(&record).unwrap();

    println!("\nserialized record:\n {:?}\n", serialized_record.len());

    let record_components = RecordSerializer::<Components, EdwardsBls>::deserialize(serialized_record).unwrap();

    assert_eq!(record.serial_number_nonce, record_components.serial_number_nonce);
    assert_eq!(record.commitment_randomness, record_components.commitment_randomness);
    assert_eq!(record.birth_predicate_repr, record_components.birth_predicate_repr);
    assert_eq!(record.death_predicate_repr, record_components.death_predicate_repr);
}

#[test]
fn test_serialization_recovery() {
    //    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    let mut rng = thread_rng();

    // Generate parameters for the ledger, commitment schemes, CRH, and the
    // "always-accept" predicate.
    let circuit_parameters = InstantiatedDPC::generate_circuit_parameters(&mut rng).unwrap();
    let pred_nizk_pp = InstantiatedDPC::generate_predicate_snark_parameters(&circuit_parameters, &mut rng).unwrap();

    let pred_nizk_vk_bytes = to_bytes![
        PredicateVerificationKeyHash::hash(
            &circuit_parameters.predicate_verification_key_hash,
            &to_bytes![pred_nizk_pp.verification_key].unwrap()
        )
        .unwrap()
    ]
    .unwrap();

    let signature_parameters = &circuit_parameters.account_signature;
    let commitment_parameters = &circuit_parameters.account_commitment;

    // Generate metadata and an account for a dummy initial record.
    let meta_data = [1u8; 32];
    let dummy_account = Account::new(signature_parameters, commitment_parameters, &meta_data, &mut rng).unwrap();

    // Use genesis record, serial number, and memo to initialize the ledger.

    let sn_nonce = SerialNumberNonce::hash(&circuit_parameters.serial_number_nonce, &[0u8; 1]).unwrap();
    let record = DPC::generate_record(
        &circuit_parameters,
        &sn_nonce,
        &dummy_account.public_key,
        true,
        0,
        &RecordPayload::default(),
        &Predicate::new(pred_nizk_vk_bytes.clone()),
        &Predicate::new(pred_nizk_vk_bytes.clone()),
        &mut rng,
    )
    .unwrap();

    let commitment_randomness = record.commitment_randomness;

    // TODO (raychu86) This test fails ~ 1/4 of the time when recover_x_coordinate returns a 0 value affine incorrectly

    let commitment_randomness_bytes = to_bytes![commitment_randomness].unwrap();
    println!("commitment_randomness_bytes: {:?}", commitment_randomness_bytes);

    let (affine, iterations) = recover_from_x_coordinate::<EdwardsBls>(&commitment_randomness_bytes).unwrap();

    let recovered_bytes = recover_x_coordinate::<EdwardsBls>(affine, iterations).unwrap();

    println!("affine: {:?}", affine);
    println!("affine bytes: {:?}", to_bytes![affine].unwrap());

    println!("recovered x_coord_bytes: {:?}", recovered_bytes);

    assert_eq!(commitment_randomness_bytes, recovered_bytes);
}
