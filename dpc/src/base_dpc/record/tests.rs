// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use super::{record_encryption::*, record_serializer::*};
use crate::base_dpc::{instantiated::*, record_payload::RecordPayload, DPC};
use snarkos_curves::edwards_bls12::{EdwardsParameters, EdwardsProjective as EdwardsBls};
use snarkos_models::{algorithms::CRH, dpc::RecordSerializerScheme, objects::AccountScheme};

use snarkos_objects::{Account, AccountViewKey};

use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

pub(crate) const ITERATIONS: usize = 5;

#[test]
fn test_record_serialization() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..ITERATIONS {
        // Generate parameters for the ledger, commitment schemes, CRH, and the
        // "always-accept" program.
        let system_parameters = InstantiatedDPC::generate_system_parameters(&mut rng).unwrap();
        let noop_program_snark_pp =
            InstantiatedDPC::generate_noop_program_snark_parameters(&system_parameters, &mut rng).unwrap();

        let program_snark_vk_bytes = to_bytes![
            ProgramVerificationKeyCRH::hash(
                &system_parameters.program_verification_key_crh,
                &to_bytes![noop_program_snark_pp.verification_key].unwrap()
            )
            .unwrap()
        ]
        .unwrap();

        for _ in 0..ITERATIONS {
            let dummy_account = Account::new(
                &system_parameters.account_signature,
                &system_parameters.account_commitment,
                &system_parameters.account_encryption,
                &mut rng,
            )
            .unwrap();

            let sn_nonce_input: [u8; 32] = rng.gen();
            let value = rng.gen();
            let payload: [u8; 32] = rng.gen();

            let given_record = DPC::generate_record(
                &system_parameters,
                &SerialNumberNonce::hash(&system_parameters.serial_number_nonce, &sn_nonce_input).unwrap(),
                &dummy_account.address,
                false,
                value,
                &RecordPayload::from_bytes(&payload),
                &program_snark_vk_bytes,
                &program_snark_vk_bytes,
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
            assert_eq!(given_record.birth_program_id, record_components.birth_program_id);
            assert_eq!(given_record.death_program_id, record_components.death_program_id);
            assert_eq!(given_record.value, record_components.value);
            assert_eq!(given_record.payload, record_components.payload);
        }
    }
}

#[test]
fn test_record_encryption() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    for _ in 0..ITERATIONS {
        // Generate parameters for the ledger, commitment schemes, CRH, and the
        // "always-accept" program.
        let system_parameters = InstantiatedDPC::generate_system_parameters(&mut rng).unwrap();
        let program_snark_pp =
            InstantiatedDPC::generate_noop_program_snark_parameters(&system_parameters, &mut rng).unwrap();

        let program_snark_vk_bytes = to_bytes![
            ProgramVerificationKeyCRH::hash(
                &system_parameters.program_verification_key_crh,
                &to_bytes![program_snark_pp.verification_key].unwrap()
            )
            .unwrap()
        ]
        .unwrap();

        for _ in 0..ITERATIONS {
            let dummy_account = Account::new(
                &system_parameters.account_signature,
                &system_parameters.account_commitment,
                &system_parameters.account_encryption,
                &mut rng,
            )
            .unwrap();

            let sn_nonce_input: [u8; 32] = rng.gen();
            let value = rng.gen();
            let payload: [u8; 32] = rng.gen();

            let given_record = DPC::generate_record(
                &system_parameters,
                &SerialNumberNonce::hash(&system_parameters.serial_number_nonce, &sn_nonce_input).unwrap(),
                &dummy_account.address,
                false,
                value,
                &RecordPayload::from_bytes(&payload),
                &program_snark_vk_bytes,
                &program_snark_vk_bytes,
                &mut rng,
            )
            .unwrap();

            // Encrypt the record
            let (_, encryped_record) =
                RecordEncryption::encrypt_record(&system_parameters, &given_record, &mut rng).unwrap();
            let account_view_key = AccountViewKey::from_private_key(
                &system_parameters.account_signature,
                &system_parameters.account_commitment,
                &dummy_account.private_key,
            )
            .unwrap();

            // Decrypt the record
            let decrypted_record =
                RecordEncryption::decrypt_record(&system_parameters, &account_view_key, &encryped_record).unwrap();

            assert_eq!(given_record, decrypted_record);
        }
    }
}
