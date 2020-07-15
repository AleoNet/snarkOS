use super::record_serializer::*;
use crate::dpc::base_dpc::{instantiated::*, record_payload::RecordPayload, DPC};
use snarkos_curves::edwards_bls12::{EdwardsParameters, EdwardsProjective as EdwardsBls};
use snarkos_models::{algorithms::CRH, dpc::RecordSerializerScheme, objects::AccountScheme};

use snarkos_objects::Account;

use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

pub(crate) const ITERATIONS: usize = 10;

#[test]
fn test_record_serialization() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..5 {
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

        for _ in 0..ITERATIONS {
            let dummy_account = Account::new(
                &circuit_parameters.account_signature,
                &circuit_parameters.account_commitment,
                &circuit_parameters.account_encryption,
                &mut rng,
            )
            .unwrap();

            let sn_nonce_input: [u8; 32] = rng.gen();
            let value = rng.gen();
            let payload: [u8; 32] = rng.gen();

            let given_record = DPC::generate_record(
                &circuit_parameters,
                &SerialNumberNonce::hash(&circuit_parameters.serial_number_nonce, &sn_nonce_input).unwrap(),
                &dummy_account.address,
                true,
                value,
                &RecordPayload::from_bytes(&payload),
                &Predicate::new(pred_nizk_vk_bytes.clone()),
                &Predicate::new(pred_nizk_vk_bytes.clone()),
                &mut rng,
            )
            .unwrap();

            let (serialized_record, final_fq_high_bit) =
                RecordSerializer::<_, EdwardsParameters, EdwardsBls>::serialize(&given_record).unwrap();
            let record_components = RecordSerializer::<Components, EdwardsParameters, EdwardsBls>::deserialize(
                serialized_record,
                final_fq_high_bit,
            )
            .unwrap();

            assert_eq!(given_record.serial_number_nonce, record_components.serial_number_nonce);
            assert_eq!(
                given_record.commitment_randomness,
                record_components.commitment_randomness
            );
            assert_eq!(
                given_record.birth_predicate_hash,
                record_components.birth_predicate_hash
            );
            assert_eq!(
                given_record.death_predicate_hash,
                record_components.death_predicate_hash
            );
            assert_eq!(given_record.value, record_components.value);
            assert_eq!(given_record.payload, record_components.payload);
        }
    }
}
