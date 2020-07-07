use crate::base_dpc::{record::DPCRecord, BaseDPCComponents};
use snarkos_algorithms::signature::bytes_to_bits;
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    curves::{AffineCurve, Group, PrimeField, ProjectiveCurve},
    dpc::{DPCComponents, Record},
};
//use snarkos_objects::AccountPublicKey;
use snarkos_utilities::{to_bytes, BigInteger, FromBytes, ToBytes};

use std::marker::PhantomData;

// TODO (raychu86) resolve duplicate impls
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

//pub fn native_recover_affine_from_x_coord<G: Group + ProjectiveCurve>(
//    x_bytes: &[u8],
//) -> Result<<G as ProjectiveCurve>::Affine, DPCError> {
//    let x: <<EdwardsBls12 as ProjectiveCurve>::Affine as AffineCurve>::BaseField = FromBytes::read(x_bytes)?;
//
//    if let Some(affine) = <EdwardsBls12 as ProjectiveCurve>::Affine::from_x_coordinate(x, false) {
//        if affine.is_in_correct_subgroup_assuming_on_curve() {
//            let affine: <G as ProjectiveCurve>::Affine = FromBytes::read(&to_bytes![affine]?[..])?;
//
//            return Ok(affine);
//        }
//    }
//
//    if let Some(affine) = <EdwardsBls12 as ProjectiveCurve>::Affine::from_x_coordinate(x, true) {
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

    fn serialize(record: Self::Record) -> Result<Vec<Self::Group>, DPCError>;
}

pub struct RecordSerializer<C: BaseDPCComponents, G: Group + ProjectiveCurve>(PhantomData<C>, PhantomData<G>);

impl<C: BaseDPCComponents, G: Group + ProjectiveCurve> SerializeRecord for RecordSerializer<C, G> {
    type Group = G;
    type InnerField = <C as DPCComponents>::InnerField;
    type OuterField = <C as DPCComponents>::OuterField;
    type Record = DPCRecord<C>;

    fn serialize(record: Self::Record) -> Result<Vec<Self::Group>, DPCError> {
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
            &to_bytes![birth_predicate_repr_bits]?[..],
        )?);
        data_elements.push(recover_from_x_coordinate::<Self::Group>(
            &to_bytes![death_predicate_repr_bits]?[..],
        )?);
        data_elements.push(recover_from_x_coordinate::<Self::Group>(
            &to_bytes![birth_predicate_repr_remainder_bits]?[..],
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
                    &to_bytes![payload_field_bits]?[..],
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
                &to_bytes![payload_field_bits]?[..],
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
            &to_bytes![payload_field_bits]?[..],
        )?);

        assert_eq!(
            data_elements.len(),
            5 + num_payload_elements + (value_does_not_fit as usize) + 1
        );

        // Compute the output group elements.

        let mut output = Vec::with_capacity(data_elements.len());

        for (i, element) in data_elements.iter().enumerate() {
            output.push(element.0.into_projective());
            println!("ELEMENT {}", i);
        }

        Ok(output)
    }

    //    pub fn deserialize<G: Group + ProjectiveCurve>(serialized_record: Vec<G>) -> Result<Vec<u8>, DPCError> {
    //        let mut bytes = vec![];
    //
    //        for element in serialized_record {
    //            let affine = element.into_affine();
    //            let x = affine.to_x_coordinate();
    //            let x_bytes = to_bytes![x]?;
    //
    //            bytes.extend(x_bytes);
    //        }
    //
    //        let serialized = Self::read(&bytes[..])?;
    //
    //        println!("bytes len: {:?}", bytes.len());
    //
    //        println!("account_public_key: {:?}", to_bytes![serialized.account_public_key]?);
    //        println!("serial_number_nonce: {:?}", to_bytes![serialized.serial_number_nonce]?);
    //        println!("commitment: {:?}", to_bytes![serialized.commitment]?);
    //
    //        Ok(bytes)
    //    }
}
