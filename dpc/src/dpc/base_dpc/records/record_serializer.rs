use crate::base_dpc::{record::DPCRecord, record_payload::RecordPayload, BaseDPCComponents};
use snarkos_algorithms::encoding::Elligator2;
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH},
    curves::{AffineCurve, Group, MontgomeryModelParameters, PrimeField, ProjectiveCurve, TEModelParameters},
    dpc::{DPCComponents, Record},
};
use snarkos_utilities::{bits_to_bytes, bytes_to_bits, to_bytes, BigInteger, FromBytes, ToBytes};

use snarkos_models::curves::fp_parameters::FpParameters;

use itertools::Itertools;
use std::marker::PhantomData;

pub fn recover_from_x_coordinate<G: Group + ProjectiveCurve>(
    x_bytes: &[u8],
) -> Result<(<G as ProjectiveCurve>::Affine, bool), DPCError> {
    let g = G::Affine::from_random_bytes(&x_bytes.to_vec());
    let affine = g.unwrap();
    Ok((affine, false))
}

pub fn recover_x_coordinate<G: Group + ProjectiveCurve>(
    affine: <G as ProjectiveCurve>::Affine,
    _unused: bool,
) -> Result<Vec<u8>, DPCError> {
    Ok(to_bytes![affine.to_x_coordinate()]?)
}

pub fn encode_to_group<P: MontgomeryModelParameters + TEModelParameters, G: Group + ProjectiveCurve>(
    x_bytes: &[u8],
) -> Result<(<G as ProjectiveCurve>::Affine, bool), DPCError> {
    // TODO (howardwu): Remove this hardcoded value and use BaseField's size in bits to pad length.
    let mut bytes = x_bytes.to_vec();
    while bytes.len() < 32 {
        bytes.push(0)
    }

    let input = P::BaseField::read(&bytes[..])?;
    let (output, fq_high) = Elligator2::<P, G>::encode(&input)?;
    Ok((output, fq_high))
}

pub fn decode_from_group<P: MontgomeryModelParameters + TEModelParameters, G: Group + ProjectiveCurve>(
    affine: <G as ProjectiveCurve>::Affine,
    fq_high: bool,
) -> Result<Vec<u8>, DPCError> {
    let output = Elligator2::<P, G>::decode(&affine, fq_high)?;
    Ok(to_bytes![output]?)
}

pub trait SerializeRecord {
    type Group: Group + ProjectiveCurve;
    type InnerField: PrimeField;
    type OuterField: PrimeField;
    type Parameters: MontgomeryModelParameters + TEModelParameters;
    type Record: Record;
    type RecordComponents;

    const SCALAR_FIELD_BITSIZE: usize =
        <<Self::Group as Group>::ScalarField as PrimeField>::Parameters::MODULUS_BITS as usize;
    const BASE_FIELD_BITSIZE: usize = <Self::InnerField as PrimeField>::Parameters::MODULUS_BITS as usize;

    fn serialize(record: &Self::Record) -> Result<(Vec<Self::Group>, bool), DPCError>;

    fn deserialize(
        serialized_record: Vec<Self::Group>,
        final_fq_high_bit: bool,
    ) -> Result<Self::RecordComponents, DPCError>;
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

pub struct RecordSerializer<
    C: BaseDPCComponents,
    P: MontgomeryModelParameters + TEModelParameters,
    G: Group + ProjectiveCurve,
>(PhantomData<C>, PhantomData<P>, PhantomData<G>);

impl<C: BaseDPCComponents, P: MontgomeryModelParameters + TEModelParameters, G: Group + ProjectiveCurve> SerializeRecord
    for RecordSerializer<C, P, G>
{
    type Group = G;
    type InnerField = <C as DPCComponents>::InnerField;
    type OuterField = <C as DPCComponents>::OuterField;
    type Parameters = P;
    type Record = DPCRecord<C>;
    type RecordComponents = RecordComponents<C>;

    /// Records are serialized in a specialized format to be space-saving.
    ///
    fn serialize(record: &Self::Record) -> Result<(Vec<Self::Group>, bool), DPCError> {
        let base_field_bitsize = <Self::InnerField as PrimeField>::size_in_bits();
        let outer_field_bitsize = <Self::OuterField as PrimeField>::size_in_bits();

        // A standard unit for packing bits into data storage
        let data_field_bitsize = base_field_bitsize - 1;

        // Assumption 1 - The scalar field bit size must be strictly less than the base field bit size
        // for the logic below to work correctly.
        assert!(Self::SCALAR_FIELD_BITSIZE < base_field_bitsize);

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

        // Create the vector for storing data elements.

        let mut data_elements = vec![];
        let mut fq_high_bits = vec![];

        // These elements are already in the constraint field.

        let serial_number_nonce = record.serial_number_nonce();
        let (serial_number_nonce_encoded, fq_high) =
            recover_from_x_coordinate::<Self::Group>(&to_bytes![serial_number_nonce]?[..])?;

        data_elements.push(serial_number_nonce_encoded);
        fq_high_bits.push(fq_high);

        assert_eq!(data_elements.len(), 1);
        assert_eq!(fq_high_bits.len(), 1);

        // These elements need to be represented in the constraint field.

        let commitment_randomness = record.commitment_randomness();
        let birth_predicate_repr = record.birth_predicate_repr();
        let death_predicate_repr = record.death_predicate_repr();
        let payload = record.payload();
        let value = record.value();

        // Process commitment_randomness. (Assumption 1 applies)

        let (encoded_commitment_randomness, fq_high) =
            encode_to_group::<Self::Parameters, Self::Group>(&to_bytes![commitment_randomness]?[..])?;
        data_elements.push(encoded_commitment_randomness);
        fq_high_bits.push(fq_high);

        assert_eq!(data_elements.len(), 2);
        assert_eq!(fq_high_bits.len(), 2);

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

        let (encoded_birth_predicate_repr, fq_high) =
            encode_to_group::<Self::Parameters, Self::Group>(&bits_to_bytes(&birth_predicate_repr_bits)[..])?;
        data_elements.push(encoded_birth_predicate_repr);
        fq_high_bits.push(fq_high);

        let (encoded_death_predicate_repr, fq_high) =
            encode_to_group::<Self::Parameters, Self::Group>(&bits_to_bytes(&death_predicate_repr_bits)[..])?;
        data_elements.push(encoded_death_predicate_repr);
        fq_high_bits.push(fq_high);

        let (encoded_birth_predicate_repr_remainder, fq_high) =
            encode_to_group::<Self::Parameters, Self::Group>(&bits_to_bytes(&birth_predicate_repr_remainder_bits)[..])?;
        data_elements.push(encoded_birth_predicate_repr_remainder);
        fq_high_bits.push(fq_high);

        assert_eq!(data_elements.len(), 5);
        assert_eq!(fq_high_bits.len(), 5);

        // Process payload.

        let payload_bytes = to_bytes![payload]?;
        let payload_bits = bytes_to_bits(&payload_bytes);

        let mut payload_field_bits = Vec::with_capacity(payload_field_bitsize + 1);

        for (i, bit) in payload_bits.iter().enumerate() {
            payload_field_bits.push(*bit);

            if (i > 0) && ((i + 1) % payload_field_bitsize == 0) {
                // (Assumption 4)
                payload_field_bits.push(true);
                let (encoded_payload_field, fq_high) =
                    encode_to_group::<Self::Parameters, Self::Group>(&bits_to_bytes(&payload_field_bits)[..])?;

                data_elements.push(encoded_payload_field);
                fq_high_bits.push(fq_high);

                payload_field_bits.clear();
            }
        }

        let num_payload_elements = payload_bits.len() / payload_field_bitsize;
        assert_eq!(data_elements.len(), 5 + num_payload_elements);
        assert_eq!(fq_high_bits.len(), 5 + num_payload_elements);

        // Process payload remainder and value.

        // Determine if value can fit in current payload_field_bits.
        let value_does_not_fit = (payload_field_bits.len() + fq_high_bits.len() + (std::mem::size_of_val(&value) * 8))
            > payload_field_bitsize;

        if value_does_not_fit {
            // (Assumption 4)
            payload_field_bits.push(true);

            let (encoded_payload_field, fq_high) =
                encode_to_group::<Self::Parameters, Self::Group>(&bits_to_bytes(&payload_field_bits)[..])?;

            data_elements.push(encoded_payload_field);
            fq_high_bits.push(fq_high);

            payload_field_bits.clear();
        }

        assert_eq!(
            data_elements.len(),
            5 + num_payload_elements + (value_does_not_fit as usize)
        );

        // Append the value bits and create the final base element.
        let value_bits = bytes_to_bits(&to_bytes![value]?);

        // (Assumption 4)
        let fq_high_and_payload_and_value_bits = [vec![true], fq_high_bits, value_bits, payload_field_bits].concat();

        let (encoded_fq_high_and_payload_and_value_field, final_fq_high_bit) =
            encode_to_group::<Self::Parameters, Self::Group>(&bits_to_bytes(&fq_high_and_payload_and_value_bits)[..])?;

        data_elements.push(encoded_fq_high_and_payload_and_value_field);

        assert_eq!(
            data_elements.len(),
            5 + num_payload_elements + (value_does_not_fit as usize) + 1
        );

        // Compute the output group elements.

        let mut output = Vec::with_capacity(data_elements.len());

        for element in data_elements.iter() {
            output.push(element.into_projective());
        }

        Ok((output, final_fq_high_bit))
    }

    fn deserialize(
        serialized_record: Vec<Self::Group>,
        final_fq_high_bit: bool,
    ) -> Result<Self::RecordComponents, DPCError> {
        let base_field_bitsize = <Self::InnerField as PrimeField>::size_in_bits();
        let outer_field_bitsize = <Self::OuterField as PrimeField>::size_in_bits();

        let data_field_bitsize = base_field_bitsize - 1;
        let remainder_size = outer_field_bitsize - data_field_bitsize;

        let payload_field_bitsize = data_field_bitsize - 1;

        // Extract the fq_bits
        let final_element = &serialized_record[serialized_record.len() - 1];
        let final_element_bytes =
            decode_from_group::<Self::Parameters, Self::Group>(final_element.into_affine(), final_fq_high_bit)?;
        let final_element_bits = bytes_to_bits(&final_element_bytes);

        let fq_high_bits = &final_element_bits[1..serialized_record.len()];

        // Deserialize serial number nonce

        let (serial_number_nonce, serial_number_nonce_fq_high) = &(serialized_record[0], fq_high_bits[0]);
        let serial_number_nonce_bytes =
            recover_x_coordinate::<Self::Group>(serial_number_nonce.into_affine(), *serial_number_nonce_fq_high)?;
        let serial_number_nonce = <C::SerialNumberNonceCRH as CRH>::Output::read(&serial_number_nonce_bytes[..])?;

        // Deserialize commitment randomness

        let (commitment_randomness, commitment_randomness_fq_high) = &(serialized_record[1], fq_high_bits[1]);
        let commitment_randomness_bytes = decode_from_group::<Self::Parameters, Self::Group>(
            commitment_randomness.into_affine(),
            *commitment_randomness_fq_high,
        )?;
        let commitment_randomness_bits = &bytes_to_bits(&commitment_randomness_bytes)[0..data_field_bitsize];

        let commitment_randomness = <C::RecordCommitment as CommitmentScheme>::Randomness::read(
            &bits_to_bytes(commitment_randomness_bits)[..],
        )?;

        // Deserialize birth and death predicates

        let (birth_predicate_repr, birth_pred_repr_fq_high) = &(serialized_record[2], fq_high_bits[2]);

        let (death_predicate_repr, death_pred_repr_fq_high) = &(serialized_record[3], fq_high_bits[3]);

        let (predicate_repr_remainder, pred_repr_remainder_fq_high) = &(serialized_record[4], fq_high_bits[4]);

        let birth_predicate_repr_bytes = decode_from_group::<Self::Parameters, Self::Group>(
            birth_predicate_repr.into_affine(),
            *birth_pred_repr_fq_high,
        )?;
        let death_predicate_repr_bytes = decode_from_group::<Self::Parameters, Self::Group>(
            death_predicate_repr.into_affine(),
            *death_pred_repr_fq_high,
        )?;
        let predicate_repr_remainder_bytes = decode_from_group::<Self::Parameters, Self::Group>(
            predicate_repr_remainder.into_affine(),
            *pred_repr_remainder_fq_high,
        )?;

        let mut birth_predicate_repr_bits = bytes_to_bits(&birth_predicate_repr_bytes)[0..data_field_bitsize].to_vec();
        let mut death_predicate_repr_bits = bytes_to_bits(&death_predicate_repr_bytes)[0..data_field_bitsize].to_vec();

        let predicate_repr_remainder_bits = bytes_to_bits(&predicate_repr_remainder_bytes);
        birth_predicate_repr_bits.extend(&predicate_repr_remainder_bits[0..remainder_size]);
        death_predicate_repr_bits.extend(&predicate_repr_remainder_bits[remainder_size..remainder_size * 2]);

        let birth_predicate_repr = bits_to_bytes(&birth_predicate_repr_bits);
        let death_predicate_repr = bits_to_bytes(&death_predicate_repr_bits);

        // Deserialize the value

        let value_start = serialized_record.len();
        let value_end = value_start + (std::mem::size_of_val(&<Self::Record as Record>::Value::default()) * 8);
        let value: <Self::Record as Record>::Value =
            FromBytes::read(&bits_to_bytes(&final_element_bits[value_start..value_end])[..])?;

        // Deserialize payload

        let mut payload_bits = vec![];
        for (element, fq_high) in serialized_record[5..serialized_record.len() - 1]
            .iter()
            .zip_eq(&fq_high_bits[5..])
        {
            let element_bytes = decode_from_group::<Self::Parameters, Self::Group>(element.into_affine(), *fq_high)?;
            payload_bits.extend_from_slice(&bytes_to_bits(&element_bytes));
        }

        println!("payload_bits len: {:?}", payload_bits.len());
        payload_bits.pop();
        payload_bits.pop();
        payload_bits.pop();
        payload_bits.pop();
        payload_bits.pop();

        println!("payload_bits len after pop: {:?}", payload_bits.len());

        payload_bits.extend_from_slice(&final_element_bits[value_end..]);

        let payload = RecordPayload::read(&bits_to_bytes(&payload_bits)[..])?;

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
