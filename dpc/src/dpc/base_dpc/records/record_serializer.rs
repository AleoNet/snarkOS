use crate::base_dpc::{record::DPCRecord, record_payload::RecordPayload, BaseDPCComponents};
use snarkos_algorithms::crh::bytes_to_bits;
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    curves::{AffineCurve, Group, PrimeField, ProjectiveCurve},
    dpc::{DPCComponents, Record},
};
//use snarkos_objects::AccountPublicKey;
use snarkos_utilities::{to_bytes, BigInteger, FromBytes, ToBytes};

use std::marker::PhantomData;

pub fn recover_from_x_coordinate<G: Group + ProjectiveCurve>(
    x_bytes: &[u8],
) -> Result<(<G as ProjectiveCurve>::Affine, u32), DPCError> {
    let mut bytes = x_bytes.to_vec();
    let mut iterations = 0u32;

    let mut g = G::Affine::from_random_bytes(&bytes);
    while g.is_none() {
        bytes.iter_mut().for_each(|i| *i = i.wrapping_sub(1));
        g = G::Affine::from_random_bytes(&bytes);
        iterations += 1;
    }

    println!("\tITERATIONS - {}", iterations);
    let affine = g.unwrap();
    Ok((affine, iterations))

    //    let affine = g.unwrap();
    //    if affine.is_in_correct_subgroup_assuming_on_curve() {
    //        return Ok((affine, iterations));
    //    }
    //
    //    Err(DPCError::Message("NotInCorrectSubgroupOnCurve".into()))
}

pub fn recover_x_coordinate<G: Group + ProjectiveCurve>(
    affine: <G as ProjectiveCurve>::Affine,
    iterations: u32,
) -> Result<Vec<u8>, DPCError> {
    let mut x_bytes = to_bytes![affine.to_x_coordinate()]?;

    for _ in 0..iterations {
        x_bytes.iter_mut().for_each(|i| *i = i.wrapping_add(1));
    }

    Ok(x_bytes)
}

//TODO (raychu86) move bits_to_bytes and bytes_to_bits into utilities
pub fn bits_to_bytes(bits: &[bool]) -> Vec<u8> {
    // Pad the bits if it not a correct size
    let mut bits = bits.to_vec();
    if bits.len() % 8 != 0 {
        let current_length = bits.len();
        for _ in 0..(8 - current_length % 8) {
            bits.push(false);
        }
    }

    let mut bytes = Vec::with_capacity(bits.len() / 8);
    for bits in bits.chunks(8) {
        let mut result = 0u8;
        for (i, bit) in bits.iter().enumerate() {
            let bit_value = *bit as u8;
            result = result + (bit_value << i as u8);
        }
        bytes.push(result);
    }
    bytes
}

//pub fn native_recover_affine_from_x_coord<G: Group + ProjectiveCurve>(
//    x_bytes: &[u8],
//) -> Result<<G as ProjectiveCurve>::Affine, DPCError> {
//    let x: <<G as ProjectiveCurve>::Affine as AffineCurve>::BaseField = FromBytes::read(x_bytes)?;
//
//    if let Some(affine) = <G as ProjectiveCurve>::Affine::from_x_coordinate(x, false) {
//        if affine.is_in_correct_subgroup_assuming_on_curve() {
//            let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//
//            return Ok(affine);
//        }
//    }
//
//    if let Some(affine) = <G as ProjectiveCurve>::Affine::from_x_coordinate(x, true) {
//        if affine.is_in_correct_subgroup_assuming_on_curve() {
//            let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//
//            return Ok(affine);
//        }
//    }
//
//    Err(BindingSignatureError::NotInCorrectSubgroupOnCurve(to_bytes![x]?))
//}

pub trait SerializeRecord {
    type Group: Group + ProjectiveCurve;
    type InnerField: PrimeField;
    type OuterField: PrimeField;
    type Record: Record;
    type RecordComponents;

    fn serialize(record: Self::Record) -> Result<Vec<(Self::Group, u32)>, DPCError>;

    fn deserialize(serialized_record: Vec<(Self::Group, u32)>) -> Result<Self::RecordComponents, DPCError>;
}

pub struct RecordComponents<C: BaseDPCComponents> {
    //    pub(super) value: u64,
    pub(super) payload: RecordPayload,

    pub(super) birth_predicate_repr: Vec<u8>,
    pub(super) death_predicate_repr: Vec<u8>,

    //    pub(super) serial_number_nonce: <C::SerialNumberNonceCRH as CRH>::Output,

    //    pub(super) commitment_randomness: <C::RecordCommitment as CommitmentScheme>::Randomness,
    pub(super) _components: PhantomData<C>,
}

pub struct RecordSerializer<C: BaseDPCComponents, G: Group + ProjectiveCurve>(PhantomData<C>, PhantomData<G>);

impl<C: BaseDPCComponents, G: Group + ProjectiveCurve> SerializeRecord for RecordSerializer<C, G> {
    type Group = G;
    type InnerField = <C as DPCComponents>::InnerField;
    type OuterField = <C as DPCComponents>::OuterField;
    type Record = DPCRecord<C>;
    type RecordComponents = RecordComponents<C>;

    fn serialize(record: Self::Record) -> Result<Vec<(Self::Group, u32)>, DPCError> {
        let scalar_field_bitsize = <Self::Group as Group>::ScalarField::size_in_bits();
        let base_field_bitsize = <Self::InnerField as PrimeField>::size_in_bits();
        let outer_field_bitsize = <Self::OuterField as PrimeField>::size_in_bits();

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

        // Create the vector for storing data elements.

        let mut data_elements = vec![];

        // These elements are already in the constraint field.

        let serial_number_nonce = record.serial_number_nonce();
        data_elements.push(recover_from_x_coordinate::<Self::Group>(
            &to_bytes![serial_number_nonce]?[..],
        )?);

        assert_eq!(data_elements.len(), 1);

        // These elements need to be represented in the constraint field.

        let commitment_randomness = record.commitment_randomness();
        let birth_predicate_repr = record.birth_predicate_repr();
        let death_predicate_repr = record.death_predicate_repr();
        let payload = record.payload();
        let value = record.value();

        // Process commitment_randomness. (Assumption 1 applies)

        data_elements.push(recover_from_x_coordinate::<Self::Group>(
            &to_bytes![commitment_randomness]?[..],
        )?);

        assert_eq!(data_elements.len(), 2);

        // Process birth_predicate_repr and death_predicate_repr. (Assumption 2 and 3 applies)

        let birth_predicate_repr_biginteger = Self::OuterField::read(&birth_predicate_repr[..])?.into_repr();
        let death_predicate_repr_biginteger = Self::OuterField::read(&death_predicate_repr[..])?.into_repr();

        println!(
            "\nbirth_predicate_repr_biginteger: {:?}",
            birth_predicate_repr_biginteger
        );
        println!("birth_predicate_repr: {:?}", birth_predicate_repr);
        println!("birth_predicate_repr_biginteger to bytes: {:?}\n", to_bytes![
            birth_predicate_repr_biginteger
        ]?);

        let mut birth_predicate_repr_bits = Vec::with_capacity(base_field_bitsize);
        let mut death_predicate_repr_bits = Vec::with_capacity(base_field_bitsize);
        let mut birth_predicate_repr_remainder_bits = Vec::with_capacity(outer_field_bitsize - data_field_bitsize);
        let mut death_predicate_repr_remainder_bits = Vec::with_capacity(outer_field_bitsize - data_field_bitsize);

        for i in 0..data_field_bitsize {
            birth_predicate_repr_bits.push(birth_predicate_repr_biginteger.get_bit(i));
            death_predicate_repr_bits.push(death_predicate_repr_biginteger.get_bit(i));
        }

        // (Assumption 2 applies)
        for i in data_field_bitsize..outer_field_bitsize {
            birth_predicate_repr_remainder_bits.push(birth_predicate_repr_biginteger.get_bit(i));
            death_predicate_repr_remainder_bits.push(death_predicate_repr_biginteger.get_bit(i));
        }
        birth_predicate_repr_remainder_bits.extend_from_slice(&death_predicate_repr_remainder_bits);

        // (Assumption 3 applies)
        data_elements.push(recover_from_x_coordinate::<Self::Group>(
            &bits_to_bytes(&birth_predicate_repr_bits)[..],
        )?);

        data_elements.push(recover_from_x_coordinate::<Self::Group>(
            &bits_to_bytes(&death_predicate_repr_bits)[..],
        )?);
        data_elements.push(recover_from_x_coordinate::<Self::Group>(
            &bits_to_bytes(&birth_predicate_repr_remainder_bits)[..],
        )?);

        assert_eq!(data_elements.len(), 5);

        // Process payload.

        let payload_bytes = to_bytes![payload]?;
        let payload_bits = bytes_to_bits(&payload_bytes);

        let mut payload_field_bits = Vec::with_capacity(data_field_bitsize);

        for (i, bit) in payload_bits.iter().enumerate() {
            payload_field_bits.push(*bit);

            if i > 0 && i % data_field_bitsize == 0 {
                data_elements.push(recover_from_x_coordinate::<Self::Group>(
                    &bits_to_bytes(&payload_field_bits)[..],
                )?);
                payload_field_bits.clear();
            }
        }

        let num_payload_elements = payload_bits.len() / data_field_bitsize;
        assert_eq!(data_elements.len(), 5 + num_payload_elements);

        // Process payload remainder and value.

        // Determine if value can fit in current payload_field_bits.
        let value_does_not_fit = (payload_field_bits.len() + std::mem::size_of_val(&value)) > data_field_bitsize;

        if value_does_not_fit {
            data_elements.push(recover_from_x_coordinate::<Self::Group>(
                &bits_to_bytes(&payload_field_bits)[..],
            )?);
            payload_field_bits.clear();
        }

        assert_eq!(
            data_elements.len(),
            5 + num_payload_elements + (value_does_not_fit as usize)
        );

        // Append the value bits and create the final base element.
        let value_bits = bytes_to_bits(&to_bytes![value]?);
        payload_field_bits.extend_from_slice(&value_bits);

        data_elements.push(recover_from_x_coordinate::<Self::Group>(
            &bits_to_bytes(&payload_field_bits)[..],
        )?);

        assert_eq!(
            data_elements.len(),
            5 + num_payload_elements + (value_does_not_fit as usize) + 1
        );

        // Compute the output group elements.

        let mut output = Vec::with_capacity(data_elements.len());

        for (i, element) in data_elements.iter().enumerate() {
            output.push((element.0.into_projective(), element.1));
            println!("ELEMENT {}", i);
        }

        {
            println!("----CHECK BASIC RECOVERY----\n");
            // (Temporary) check that the birth and death predicate repr bytes can be recovered correctly

            let birth_predicate_repr_bytes = bits_to_bytes(&birth_predicate_repr_bits);

            let (affine, iterations) =
                recover_from_x_coordinate::<Self::Group>(&birth_predicate_repr_bytes[..]).unwrap();

            let recovered_birth_predicate_repr = recover_x_coordinate::<Self::Group>(affine, iterations).unwrap();

            println!("birth_predicate_repr_bits: {:?}", birth_predicate_repr_bits);
            println!(
                "recovered_birth_predicate_repr_bits: {:?}",
                bytes_to_bits(&recovered_birth_predicate_repr)
            );

            assert_eq!(birth_predicate_repr_bytes, recovered_birth_predicate_repr);

            let death_predicate_repr_bytes = bits_to_bytes(&death_predicate_repr_bits);

            let (affine, iterations) =
                recover_from_x_coordinate::<Self::Group>(&death_predicate_repr_bytes[..]).unwrap();

            let recovered_death_predicate_repr = recover_x_coordinate::<Self::Group>(affine, iterations).unwrap();

            assert_eq!(death_predicate_repr_bytes, recovered_death_predicate_repr);

            let birth_predicate_repr_remainder_bytes = bits_to_bytes(&birth_predicate_repr_remainder_bits);

            let (affine, iterations) =
                recover_from_x_coordinate::<Self::Group>(&birth_predicate_repr_remainder_bytes[..]).unwrap();

            let recovered_remainder = recover_x_coordinate::<Self::Group>(affine, iterations).unwrap();

            assert_eq!(birth_predicate_repr_remainder_bytes, recovered_remainder);
        }

        Ok(output)
    }

    fn deserialize(serialized_record: Vec<(Self::Group, u32)>) -> Result<Self::RecordComponents, DPCError> {
        let scalar_field_bitsize = <Self::Group as Group>::ScalarField::size_in_bits();
        let base_field_bitsize = <Self::InnerField as PrimeField>::size_in_bits();
        let outer_field_bitsize = <Self::OuterField as PrimeField>::size_in_bits();

        let data_field_bitsize = base_field_bitsize - 1;
        let remainder_size = outer_field_bitsize - data_field_bitsize;

        //        let mut bytes = vec![];
        //        for element in serialized_record {
        //            let affine = element.into_affine();
        //            let x = affine.to_x_coordinate();
        //            let x_bytes = to_bytes![x]?;
        //
        //            bytes.extend(x_bytes);
        //        }

        let birth_pred_repr_affine = serialized_record[2].0.into_affine();
        let birth_pred_repr_iterations = serialized_record[2].1;

        let death_pred_repr_affine = serialized_record[3].0.into_affine();
        let death_pred_repr_iterations = serialized_record[3].1;

        let pred_repr_remainder_affine = serialized_record[4].0.into_affine();
        let pred_repr_remainder_iterations = serialized_record[4].1;

        let recovered_birth_pred_repr =
            recover_x_coordinate::<Self::Group>(birth_pred_repr_affine, birth_pred_repr_iterations)?;
        let recovered_death_pred_repr =
            recover_x_coordinate::<Self::Group>(death_pred_repr_affine, death_pred_repr_iterations)?;
        let recovered_pred_repr_remainder =
            recover_x_coordinate::<Self::Group>(pred_repr_remainder_affine, pred_repr_remainder_iterations)?;

        let mut recovered_birth_pred_repr_bits =
            bytes_to_bits(&recovered_birth_pred_repr)[0..base_field_bitsize].to_vec();
        let mut recovered_death_pred_repr_bits =
            bytes_to_bits(&recovered_death_pred_repr)[0..base_field_bitsize].to_vec();

        let recovered_pred_repr_remainder_bits = bytes_to_bits(&recovered_pred_repr_remainder);

        let no_remainder_birth_predicate_repr = bits_to_bytes(&recovered_birth_pred_repr_bits);

        println!(
            "pre remainder recovered birth_predicate_repr: {:?}",
            no_remainder_birth_predicate_repr
        );

        recovered_birth_pred_repr_bits.extend(&recovered_pred_repr_remainder_bits[0..remainder_size]);
        recovered_death_pred_repr_bits.extend(&recovered_pred_repr_remainder_bits[remainder_size..remainder_size * 2]);

        let birth_predicate_repr = bits_to_bytes(&recovered_birth_pred_repr_bits);
        let death_predicate_repr = bits_to_bytes(&recovered_death_pred_repr_bits);

        println!("recovered birth_predicate_repr: {:?}", birth_predicate_repr);
        println!("recovered death_predicate_repr: {:?}", death_predicate_repr);

        //        println!("recovered_birth_pred_repr_1_bits: {:?}", recovered_birth_pred_repr_1_bits);
        //        println!("recovered_death_pred_repr_1_bits: {:?}", recovered_death_pred_repr_1_bits);

        Ok(RecordComponents {
            payload: RecordPayload::default(),
            birth_predicate_repr: vec![],
            death_predicate_repr: vec![],
            _components: PhantomData,
        })
    }
}
