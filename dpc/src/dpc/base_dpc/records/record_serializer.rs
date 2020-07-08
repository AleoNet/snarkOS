use crate::base_dpc::{record::DPCRecord, record_payload::RecordPayload, BaseDPCComponents};
use snarkos_algorithms::crh::bytes_to_bits;
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
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

pub trait SerializeRecord {
    type Group: Group + ProjectiveCurve;
    type InnerField: PrimeField;
    type OuterField: PrimeField;
    type Record: Record;
    type RecordComponents;

    fn serialize(record: &Self::Record) -> Result<Vec<(Self::Group, u32)>, DPCError>;

    fn deserialize(serialized_record: Vec<(Self::Group, u32)>) -> Result<Self::RecordComponents, DPCError>;
}

pub struct RecordComponents<C: BaseDPCComponents> {
    pub value: u64,
    pub payload: RecordPayload,

    pub birth_predicate_repr: Vec<u8>,
    pub death_predicate_repr: Vec<u8>,

    pub serial_number_nonce: <C::SerialNumberNonceCRH as CRH>::Output,

    pub commitment_randomness: <C::RecordCommitment as CommitmentScheme>::Randomness,
    pub _components: PhantomData<C>,
}

pub struct RecordSerializer<C: BaseDPCComponents, G: Group + ProjectiveCurve>(PhantomData<C>, PhantomData<G>);

impl<C: BaseDPCComponents, G: Group + ProjectiveCurve> SerializeRecord for RecordSerializer<C, G> {
    type Group = G;
    type InnerField = <C as DPCComponents>::InnerField;
    type OuterField = <C as DPCComponents>::OuterField;
    type Record = DPCRecord<C>;
    type RecordComponents = RecordComponents<C>;

    fn serialize(record: &Self::Record) -> Result<Vec<(Self::Group, u32)>, DPCError> {
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
        }

        Ok(output)
    }

    fn deserialize(serialized_record: Vec<(Self::Group, u32)>) -> Result<Self::RecordComponents, DPCError> {
        let base_field_bitsize = <Self::InnerField as PrimeField>::size_in_bits();
        let outer_field_bitsize = <Self::OuterField as PrimeField>::size_in_bits();

        let data_field_bitsize = base_field_bitsize - 1;
        let remainder_size = outer_field_bitsize - data_field_bitsize;

        // Deserialize serial number nonce

        let (serial_number_nonce, serial_number_nonce_iterations) = &serialized_record[0];
        let serial_number_nonce_bytes =
            recover_x_coordinate::<Self::Group>(serial_number_nonce.into_affine(), *serial_number_nonce_iterations)?;
        let serial_number_nonce_bits = &bytes_to_bits(&serial_number_nonce_bytes)[0..data_field_bitsize];

        let serial_number_nonce =
            <C::SerialNumberNonceCRH as CRH>::Output::read(&bits_to_bytes(serial_number_nonce_bits)[..])?;

        // Deserialize commitment randomness

        let (commitment_randomness, commitment_randomness_iterations) = &serialized_record[1];
        let commitment_randomness_bytes = recover_x_coordinate::<Self::Group>(
            commitment_randomness.into_affine(),
            *commitment_randomness_iterations,
        )?;
        let commitment_randomness_bits = &bytes_to_bits(&commitment_randomness_bytes)[0..data_field_bitsize];

        let commitment_randomness = <C::RecordCommitment as CommitmentScheme>::Randomness::read(
            &bits_to_bytes(commitment_randomness_bits)[..],
        )?;

        // Deserialize birth and death predicates

        let (birth_predicate_repr, birth_pred_repr_iterations) = &serialized_record[2];

        let (death_predicate_repr, death_pred_repr_iterations) = &serialized_record[3];

        let (predicate_repr_remainder, pred_repr_remainder_iterations) = &serialized_record[4];

        let birth_predicate_repr_bytes =
            recover_x_coordinate::<Self::Group>(birth_predicate_repr.into_affine(), *birth_pred_repr_iterations)?;
        let death_predicate_repr_bytes =
            recover_x_coordinate::<Self::Group>(death_predicate_repr.into_affine(), *death_pred_repr_iterations)?;
        let predicate_repr_remainder_bytes = recover_x_coordinate::<Self::Group>(
            predicate_repr_remainder.into_affine(),
            *pred_repr_remainder_iterations,
        )?;

        let mut birth_predicate_repr_bits = bytes_to_bits(&birth_predicate_repr_bytes)[0..data_field_bitsize].to_vec();
        let mut death_predicate_repr_bits = bytes_to_bits(&death_predicate_repr_bytes)[0..data_field_bitsize].to_vec();

        let predicate_repr_remainder_bits = bytes_to_bits(&predicate_repr_remainder_bytes);
        birth_predicate_repr_bits.extend(&predicate_repr_remainder_bits[0..remainder_size]);
        death_predicate_repr_bits.extend(&predicate_repr_remainder_bits[remainder_size..remainder_size * 2]);

        let birth_predicate_repr = bits_to_bytes(&birth_predicate_repr_bits);
        let death_predicate_repr = bits_to_bytes(&death_predicate_repr_bits);

        // Deserialize payload

        let mut payload_and_value_bits = vec![];
        for (element, iterations) in serialized_record[5..].iter() {
            let element_bytes = recover_x_coordinate::<Self::Group>(element.into_affine(), *iterations)?;
            payload_and_value_bits.extend(&bytes_to_bits(&element_bytes)[0..data_field_bitsize]);
        }

        let payload_and_value_bytes = bits_to_bytes(&payload_and_value_bits);

        let payload = RecordPayload::read(&payload_and_value_bytes[..])?;
        let value: u64 = FromBytes::read(&payload_and_value_bytes[payload.size()..])?;

        Ok(RecordComponents {
            value,
            payload,
            birth_predicate_repr,
            death_predicate_repr,
            serial_number_nonce,
            commitment_randomness,
            _components: PhantomData,
        })
    }
}
