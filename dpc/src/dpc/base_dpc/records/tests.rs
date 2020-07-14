use super::record_serializer::*;
use crate::dpc::base_dpc::{instantiated::*, record_payload::RecordPayload, DPC};
use snarkos_curves::edwards_bls12::{EdwardsParameters, EdwardsProjective as EdwardsBls};
use snarkos_models::{algorithms::CRH, objects::AccountScheme};

use snarkos_objects::Account;

use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

pub(crate) const ITERATIONS: usize = 100;

#[test]
fn test_record_serialization() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

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
    let encryption_parameters = &circuit_parameters.account_encryption;

    for _ in 0..ITERATIONS {
        let dummy_account = Account::new(
            signature_parameters,
            commitment_parameters,
            encryption_parameters,
            &mut rng,
        )
        .unwrap();

        let sn_nonce = SerialNumberNonce::hash(&circuit_parameters.serial_number_nonce, &[0u8; 1]).unwrap();
        let record = DPC::generate_record(
            &circuit_parameters,
            &sn_nonce,
            &dummy_account.address,
            true,
            0,
            &RecordPayload::default(),
            &Predicate::new(pred_nizk_vk_bytes.clone()),
            &Predicate::new(pred_nizk_vk_bytes.clone()),
            &mut rng,
        )
        .unwrap();

        let serialized_record = RecordSerializer::<_, EdwardsParameters, EdwardsBls>::serialize(&record).unwrap();

        let record_components =
            RecordSerializer::<Components, EdwardsParameters, EdwardsBls>::deserialize(serialized_record).unwrap();

        assert_eq!(record.serial_number_nonce, record_components.serial_number_nonce);
        assert_eq!(record.commitment_randomness, record_components.commitment_randomness);
        assert_eq!(record.birth_predicate_repr, record_components.birth_predicate_repr);
        assert_eq!(record.death_predicate_repr, record_components.death_predicate_repr);
        assert_eq!(record.payload, record_components.payload);
        assert_eq!(record.value, record_components.value);
    }
}
