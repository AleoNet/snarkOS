use super::record_serializer::*;
use crate::dpc::base_dpc::{instantiated::*, record_payload::RecordPayload, DPC};
use snarkos_algorithms::crh::bytes_to_bits;
use snarkos_curves::edwards_bls12::{EdwardsParameters, EdwardsProjective as EdwardsBls};
use snarkos_models::{algorithms::CRH, objects::AccountScheme};

use snarkos_objects::Account;

use snarkos_utilities::{bytes::ToBytes, to_bytes};

//use std::io::Cursor;

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
    let encryption_parameters = &circuit_parameters.account_encryption;

    // Generate metadata and an account for a dummy initial record.
    let dummy_account = Account::new(
        signature_parameters,
        commitment_parameters,
        encryption_parameters,
        &mut rng,
    )
    .unwrap();

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

    let serialized_record = RecordSerializer::<_, EdwardsParameters, EdwardsBls>::serialize(&record).unwrap();

    println!("\nSerialized record length: {}\n", serialized_record.len());

    let record_components =
        RecordSerializer::<Components, EdwardsParameters, EdwardsBls>::deserialize(serialized_record).unwrap();

    println!("record.serial_number_nonce: {}", record.serial_number_nonce);
    println!(
        "record_components.serial_number_nonce: {}",
        record_components.serial_number_nonce
    );

    assert_eq!(record.serial_number_nonce, record_components.serial_number_nonce);

    println!("record.commitment_randomness: {}", record.commitment_randomness);
    println!(
        "record_components.commitment_randomness: {}",
        record_components.commitment_randomness
    );

    println!("record.birth_predicate_repr: {:?}", record.birth_predicate_repr);
    println!(
        "record_components.birth_predicate_repr: {:?}",
        record_components.birth_predicate_repr
    );

    assert_eq!(record.commitment_randomness, record_components.commitment_randomness);
    assert_eq!(record.birth_predicate_repr, record_components.birth_predicate_repr);
    assert_eq!(record.death_predicate_repr, record_components.death_predicate_repr);
    assert_eq!(record.payload, record_components.payload);
    assert_eq!(record.value, record_components.value);
}

//#[test]
//fn test_serialization_recovery() {
//    use snarkos_utilities::FromBytes;
//    use snarkos_algorithms::encoding::Elligator2;
//    use snarkos_models::curves::{Group, Field, FpParameters, PrimeField};
//    use snarkos_curves::edwards_bls12::FrParameters;
//
//    //    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
//    let mut rng = thread_rng();
//
//    // Generate parameters for the ledger, commitment schemes, CRH, and the
//    // "always-accept" predicate.
//    let circuit_parameters = InstantiatedDPC::generate_circuit_parameters(&mut rng).unwrap();
//    let pred_nizk_pp = InstantiatedDPC::generate_predicate_snark_parameters(&circuit_parameters, &mut rng).unwrap();
//
//    let pred_nizk_vk_bytes = to_bytes![
//        PredicateVerificationKeyHash::hash(
//            &circuit_parameters.predicate_verification_key_hash,
//            &to_bytes![pred_nizk_pp.verification_key].unwrap()
//        )
//        .unwrap()
//    ]
//        .unwrap();
//
//    let signature_parameters = &circuit_parameters.account_signature;
//    let commitment_parameters = &circuit_parameters.account_commitment;
//    let encryption_parameters = &circuit_parameters.account_encryption;
//
//    // Generate metadata and an account for a dummy initial record.
//    let dummy_account = Account::new(signature_parameters, commitment_parameters, encryption_parameters, &mut rng).unwrap();
//
//    // Use genesis record, serial number, and memo to initialize the ledger.
//
//    let sn_nonce = SerialNumberNonce::hash(&circuit_parameters.serial_number_nonce, &[0u8; 1]).unwrap();
//    let record = DPC::generate_record(
//        &circuit_parameters,
//        &sn_nonce,
//        &dummy_account.public_key,
//        true,
//        0,
//        &RecordPayload::default(),
//        &Predicate::new(pred_nizk_vk_bytes.clone()),
//        &Predicate::new(pred_nizk_vk_bytes.clone()),
//        &mut rng,
//    )
//        .unwrap();
//
//    let serial_number_nonce = record.serial_number_nonce;
//
//    // TODO This test fails ~ 1/4 of the time when `recover_from_x_coordinate` returns a 0 value affine incorrectly
//
//    let serial_number_nonce_bytes = to_bytes![serial_number_nonce].unwrap();
//    println!("serial_number_nonce_bytes: {:?}", serial_number_nonce_bytes);
//
//    let big_int = <<EdwardsBls as Group>::ScalarField as PrimeField>::BigInt::read(&serial_number_nonce_bytes[..]).unwrap();
//
//    let modulous = FrParameters::MODULUS;
//
//    println!("big_int < modulous: {:?}", big_int < modulous);
//
//    let scalar = <EdwardsBls as Group>::ScalarField::from_repr(big_int).unwrap();
//
//    let _scalar2 = <EdwardsBls as Group>::ScalarField::from_random_bytes(&serial_number_nonce_bytes[..]);
//
//    let encoded = Elligator2::<EdwardsParameters, EdwardsBls>::encode(&scalar).unwrap();
//    let decoded = Elligator2::<EdwardsParameters, EdwardsBls>::decode(&encoded).unwrap();
//
//    let decoded_big_int = decoded.into_repr();
//
//    let recovered_bytes = to_bytes![decoded_big_int].unwrap();
//
//    println!("recovered_bytes: {:?}", recovered_bytes);
//
////    println!("affine: {:?}", affine);
////    println!("affine bytes: {:?}", to_bytes![affine].unwrap());
////
////    assert_eq!(commitment_randomness_bytes, recovered_bytes);
//}

//
//use snarkos_models::{
//    algorithms::CommitmentScheme,
//    curves::{AffineCurve, ProjectiveCurve},
//    dpc::DPCComponents,
//};
//use snarkos_utilities::rand::UniformRand;
//
//// TODO debug iterations
//
//#[test]
//fn test_recovery_not_working_without_iteration() {
//    let rng = &mut thread_rng();
//    let commitment_randomness =
//        <<Components as DPCComponents>::RecordCommitment as CommitmentScheme>::Randomness::rand(rng);
//
//    let bytes = to_bytes![commitment_randomness].unwrap();
//
//    // This should always work, but fails a good % of the time
//    let g = <EdwardsBls as ProjectiveCurve>::Affine::from_random_bytes(&bytes);
//
//    assert!(g.is_some());
//
//    println!("g: {:?}", g.unwrap());
//}
//
//#[test]
//fn test_recovery_working_with_iteration() {
//    let rng = &mut thread_rng();
//    let commitment_randomness =
//        <<Components as DPCComponents>::RecordCommitment as CommitmentScheme>::Randomness::rand(rng);
//
//    fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
//        let mut bits = Vec::with_capacity(bytes.len() * 8);
//        for byte in bytes {
//            for i in 0..8 {
//                let bit = (*byte >> i) & 1;
//                bits.push(bit == 1)
//            }
//        }
//        bits
//    }
//
//    use snarkos_models::curves::primefield::PrimeField;
//    use snarkos_utilities::{biginteger::biginteger::BigInteger, BigInteger256};
//    //
//    //
//    //
//    let bytes = to_bytes![commitment_randomness].unwrap();
//
//    println!("{:?}", (commitment_randomness.0).0);
//    println!("{:?}", bytes_to_bits(&bytes));
//
//    println!(
//        "{} {} {} {}",
//        commitment_randomness.0.get_bit(255),
//        commitment_randomness.0.get_bit(254),
//        commitment_randomness.0.get_bit(253),
//        commitment_randomness.0.get_bit(252)
//    );
//    //
//    //
//    //    let commitment_bits = bytes_to_bits(&bytes);
//    //    println!("{:?}", commitment_bits);
//
//    //    {
//    //        let mut serialized = vec![0u8; 32];
//    //        let mut cursor = Cursor::new(&mut serialized[..]);
//    //        a.serialize_with_flags(&mut cursor, SWFlags::from_y_sign(true)).unwrap();
//    //        let mut cursor = Cursor::new(&serialized[..]);
//    //        let (b, flags) = F::deserialize_with_flags::<_, SWFlags>(&mut cursor).unwrap();
//    //        assert_eq!(flags.is_positive(), Some(true));
//    //        assert!(!flags.is_infinity());
//    //        assert_eq!(a, b);
//    //    }
//
//    //    let mut serialized = vec![0u8; 32];
//    //    let mut cursor = Cursor::new(&mut serialized[..]);
//    //    commitment_randomness.serialize(&mut cursor).unwrap();
//    use snarkos_errors::dpc::DPCError;
//    use snarkos_models::curves::Group;
//    use snarkos_utilities::FromBytes;
//
//    fn recover_affine_from_x_coord<G: Group + ProjectiveCurve>(
//        x_bytes: &BigInteger256,
//    ) -> Result<<G as ProjectiveCurve>::Affine, DPCError> {
//        let x = <<EdwardsBls as ProjectiveCurve>::Affine as AffineCurve>::BaseField::from_repr(*x_bytes).unwrap();
//
//        println!("PRINTING X - {:?}", (x.0).0);
//        println!(
//            "PRINTING X - {} {} {} {}",
//            (x.0).get_bit(255),
//            (x.0).get_bit(254),
//            (x.0).get_bit(253),
//            (x.0).get_bit(252)
//        );
//
//        //        use snarkos_models::curves::field::Field;
//
//        if let Some(affine) = <EdwardsBls as ProjectiveCurve>::Affine::from_x_coordinate(x, false) {
//            //            if affine.is_in_correct_subgroup_assuming_on_curve() {
//            //                let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//            //
//            //                return Ok(affine);
//            //            }
//            println!("IM IN THE RIGHT PLACE A");
//            let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//            return Ok(affine);
//        }
//
//        if let Some(affine) = <EdwardsBls as ProjectiveCurve>::Affine::from_x_coordinate(x, true) {
//            //            if affine.is_in_correct_subgroup_assuming_on_curve() {
//            //                let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//            //
//            //                return Ok(affine);
//            //            }
//
//            println!("IM IN THE RIGHT PLACE B");
//
//            let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//            return Ok(affine);
//        }
//
//        //        let xqr = x.square().pow((<<EdwardsBls as ProjectiveCurve>::Affine as AffineCurve>::BaseField::root_of_unity().0).0) * &x.pow((<<EdwardsBls as ProjectiveCurve>::Affine as AffineCurve>::BaseField::root_of_unity().0).0);
//
//        //        use snarkos_models::curves::Zero;
//        //
//        //        use snarkos_models::curves::One;
//        //
//        ////            let modulus_bytes = <<EdwardsBls as ProjectiveCurve>::Affine as AffineCurve>::BaseField::characteristic();
//        ////        let mut modulus_fixed_bytes = [064; 4];
//        ////        modulus_fixed_bytes.clone_from_slice(&modulus_bytes);
//        ////        let modulus = BigInteger256(modulus_fixed_bytes);
//        //////           let modulus = <<EdwardsBls as ProjectiveCurve>::Affine as AffineCurve>::BaseField::from_repr(modulus).unwrap();
//        ////
//        ////        let modulus_minus_one = modulus - &<<EdwardsBls as ProjectiveCurve>::Affine as AffineCurve>::BaseField::one();
//        ////
//        ////        let xqr = x.pow((modulus_minus_one.0).0);
//        //
//        //        let minus_one = <<EdwardsBls as ProjectiveCurve>::Affine as AffineCurve>::BaseField::zero() - &<<EdwardsBls as ProjectiveCurve>::Affine as AffineCurve>::BaseField::one();
//        //        let xqr = x.pow((minus_one.0).0);
//
//        use std::ops::Neg;
//        let xqr = x.neg();
//
//        if let Some(affine) = <EdwardsBls as ProjectiveCurve>::Affine::from_x_coordinate(xqr, false) {
//            //            if affine.is_in_correct_subgroup_assuming_on_curve() {
//            //                let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//            //
//            //                return Ok(affine);
//            //            }
//            println!("IM IN THE RIGHT PLACE C");
//            let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//            return Ok(affine);
//        }
//
//        if let Some(affine) = <EdwardsBls as ProjectiveCurve>::Affine::from_x_coordinate(xqr, true) {
//            //            if affine.is_in_correct_subgroup_assuming_on_curve() {
//            //                let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//            //
//            //                return Ok(affine);
//            //            }
//
//            println!("IM IN THE RIGHT PLACE D");
//
//            let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//            return Ok(affine);
//        }
//
//        //        if let Some(affine) = <EdwardsBls as ProjectiveCurve>::Affine::from_x_coordinate(x.double().double(), false) {
//        //            //            if affine.is_in_correct_subgroup_assuming_on_curve() {
//        //            //                let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//        //            //
//        //            //                return Ok(affine);
//        //            //            }
//        //            println!("IM IN THE RIGHT PLACE E");
//        //            let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//        //            return Ok(affine);
//        //        }
//        //
//        //        if let Some(affine) = <EdwardsBls as ProjectiveCurve>::Affine::from_x_coordinate(x.double().double(), true) {
//        //            //            if affine.is_in_correct_subgroup_assuming_on_curve() {
//        //            //                let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//        //            //
//        //            //                return Ok(affine);
//        //            //            }
//        //
//        //            println!("IM IN THE RIGHT PLACE F");
//        //
//        //            let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//        //            return Ok(affine);
//        //        }
//
//        Err(DPCError::Message("NotInCorrectSubgroupOnCurve".into()))
//    }
//
//    let g = recover_affine_from_x_coord::<EdwardsBls>(&commitment_randomness.0).unwrap();
//
//    println!("g: {:?}", g);
//
//    //    let mut serialized = to_bytes![commitment_randomness].unwrap();
//    //
//    //    let mut g = <EdwardsBls as ProjectiveCurve>::Affine::from_random_bytes(&serialized);;
//    //
//    //        let mut iterations = 0;
//    //        while g.is_none() {
//    //            serialized.iter_mut().for_each(|i| *i = i.wrapping_sub(1));
//    //            g = <EdwardsBls as ProjectiveCurve>::Affine::from_random_bytes(&serialized);
//    //            iterations += 1;
//    //        }
//    //
//    //    println!("g: {:?}", g);
//    //        println!("Iterations: {}", iterations);
//}
