use super::record_serializer::*;
use crate::dpc::base_dpc::{instantiated::*, record_payload::RecordPayload, DPC};
//use snarkos_curves::bls12_377::{Fq, Fr};
use snarkos_curves::edwards_bls12::EdwardsProjective as EdwardsBls;
use snarkos_models::{algorithms::CRH, objects::AccountScheme};

use snarkos_objects::Account;

use snarkos_utilities::{bytes::ToBytes, to_bytes};

//use rand::SeedableRng;
//use rand_xorshift::XorShiftRng;

#[test]
fn test_record_serialization() {
    //    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    use rand::thread_rng;
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
    let old_record = DPC::generate_record(
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

    let serialized_record = RecordSerializer::<_, EdwardsBls>::serialize(old_record).unwrap();

    println!("\nserialized record:\n {:?}\n", serialized_record.len());

    //    let deserialized_record = RecordSerializer::<Components>::deserialize::<EdwardsBls>(serialized_record).unwrap();
    //
    //    println!("\ndeserialized record:\n {:?}\n", deserialized_record.len());
}
