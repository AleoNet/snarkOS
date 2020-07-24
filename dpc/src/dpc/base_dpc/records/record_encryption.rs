use crate::base_dpc::{
    parameters::CircuitParameters,
    record::DPCRecord,
    records::record_serializer::*,
    BaseDPCComponents,
};
use snarkos_algorithms::encoding::Elligator2;
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    algorithms::{EncryptionScheme, CRH},
    curves::{AffineCurve, ModelParameters, ProjectiveCurve},
    dpc::{DPCComponents, Record, RecordSerializerScheme},
};
use snarkos_utilities::{bits_to_bytes, bytes_to_bits, to_bytes, FromBytes, ToBytes};

use itertools::Itertools;
use rand::Rng;
use std::marker::PhantomData;

pub struct RecordEncryption<C: BaseDPCComponents>(PhantomData<C>);

impl<C: BaseDPCComponents> RecordEncryption<C> {
    /// Encrypt the given vector of records and returns
    /// 1. Encryption Randomness
    /// 2. Encrypted record ciphertext
    /// 3. Ciphertext Selector bits - used to compress/decompress
    /// 4. Final fq high selector bit - Used to decode the plaintext
    pub fn encrypt_record<R: Rng>(
        circuit_parameters: &CircuitParameters<C>,
        record: &DPCRecord<C>,
        rng: &mut R,
    ) -> Result<
        (
            <<C as DPCComponents>::AccountEncryption as EncryptionScheme>::Randomness,
            Vec<<<C as DPCComponents>::AccountEncryption as EncryptionScheme>::Text>,
            Vec<bool>,
            bool,
        ),
        DPCError,
    > {
        // Serialize the record into group elements and fq_high bits
        let (serialized_record, final_fq_high_bit) =
            RecordSerializer::<C, C::EncryptionModelParameters, C::EncryptionGroup>::serialize(&record)?;

        let mut record_plaintexts = vec![];
        for element in serialized_record.iter() {
            // Construct the plaintext element from the serialized group elements
            // This value will be used in the inner circuit to validate the encryption
            let plaintext_element =
                <<C as DPCComponents>::AccountEncryption as EncryptionScheme>::Text::read(&to_bytes![element]?[..])?;
            record_plaintexts.push(plaintext_element);
        }

        // Encrypt the record plaintext
        let record_public_key = record.account_address().into_repr();
        let encryption_randomness = circuit_parameters
            .account_encryption
            .generate_randomness(record_public_key, rng)?;
        let record_ciphertext = circuit_parameters.account_encryption.encrypt(
            record_public_key,
            &encryption_randomness,
            &record_plaintexts,
        )?;

        // Compute the compressed ciphertext selector bits
        let mut ciphertext_selectors = vec![];
        for ciphertext_element in record_ciphertext.iter() {
            let ciphertext_element_affine =
                <C as BaseDPCComponents>::EncryptionGroup::read(&to_bytes![ciphertext_element]?[..])?.into_affine();

            let greatest =
                match <<C as BaseDPCComponents>::EncryptionGroup as ProjectiveCurve>::Affine::from_x_coordinate(
                    ciphertext_element_affine.to_x_coordinate(),
                    true,
                ) {
                    Some(affine) => ciphertext_element_affine == affine,
                    None => false,
                };

            ciphertext_selectors.push(greatest);
        }

        Ok((
            encryption_randomness,
            record_ciphertext,
            ciphertext_selectors,
            final_fq_high_bit,
        ))
    }

    // TODO (raychu86) Decrypt Ciphertext

    ///
    pub fn record_ciphertext_hash(
        circuit_parameters: &CircuitParameters<C>,
        record_ciphertext: &Vec<<<C as DPCComponents>::AccountEncryption as EncryptionScheme>::Text>,
        final_fq_high_selector: bool,
    ) -> Result<<<C as DPCComponents>::RecordCiphertextCRH as CRH>::Output, DPCError> {
        let mut ciphertext_affine_x = vec![];
        let mut selector_bits = vec![];
        for ciphertext_element in record_ciphertext.iter() {
            let ciphertext_element_affine =
                <C as BaseDPCComponents>::EncryptionGroup::read(&to_bytes![ciphertext_element]?[..])?.into_affine();
            let ciphertext_x_coordinate = ciphertext_element_affine.to_x_coordinate();

            let greatest =
                match <<C as BaseDPCComponents>::EncryptionGroup as ProjectiveCurve>::Affine::from_x_coordinate(
                    ciphertext_x_coordinate.clone(),
                    true,
                ) {
                    Some(affine) => ciphertext_element_affine == affine,
                    None => false,
                };

            selector_bits.push(greatest);
            ciphertext_affine_x.push(ciphertext_x_coordinate);
        }

        // Concatenate the ciphertext selector bits and the final fq_high selector bit
        selector_bits.push(final_fq_high_selector);
        let selector_bytes = bits_to_bytes(&selector_bits);

        Ok(circuit_parameters
            .record_ciphertext_crh
            .hash(&to_bytes![ciphertext_affine_x, selector_bytes]?)?)
    }

    pub fn prepare_encryption_gadget_components(
        circuit_parameters: &CircuitParameters<C>,
        record: &DPCRecord<C>,
        encryption_randomness: &<<C as DPCComponents>::AccountEncryption as EncryptionScheme>::Randomness,
    ) -> Result<
        (
            Vec<<C::EncryptionModelParameters as ModelParameters>::BaseField>,
            Vec<(
                <C::EncryptionModelParameters as ModelParameters>::BaseField,
                <C::EncryptionModelParameters as ModelParameters>::BaseField,
            )>,
            Vec<bool>,
            Vec<<C::AccountEncryption as EncryptionScheme>::BlindingExponent>,
        ),
        DPCError,
    > {
        // Serialize the record into group elements and fq_high bits
        let (serialized_record, final_fq_high_bit) =
            RecordSerializer::<C, C::EncryptionModelParameters, C::EncryptionGroup>::serialize(&record)?;

        // Extract the fq_bits from the serialized record
        let final_element = &serialized_record[serialized_record.len() - 1];
        let final_element_bytes = decode_from_group::<C::EncryptionModelParameters, C::EncryptionGroup>(
            final_element.into_affine(),
            final_fq_high_bit,
        )?;
        let final_element_bits = bytes_to_bits(&final_element_bytes);
        let fq_high_bits = [
            &final_element_bits[1..serialized_record.len()],
            &[final_fq_high_bit][..],
        ]
        .concat();

        let mut record_field_elements = vec![];
        let mut record_group_encoding = vec![];
        let mut record_plaintexts = vec![];

        for (i, (element, fq_high)) in serialized_record.iter().zip_eq(&fq_high_bits).enumerate() {
            let element_affine = element.into_affine();

            // Decode the field elements from the serialized group element
            // These values will be used in the inner circuit to validate bit packing and serialization
            if i == 0 {
                // Serial number nonce
                let record_field_element =
                    <<C as BaseDPCComponents>::EncryptionModelParameters as ModelParameters>::BaseField::read(
                        &to_bytes![element]?[..],
                    )?;
                record_field_elements.push(record_field_element);
            } else {
                // Decode the encoded groups into their respective field elements
                let record_field_element = Elligator2::<
                    <C as BaseDPCComponents>::EncryptionModelParameters,
                    <C as BaseDPCComponents>::EncryptionGroup,
                >::decode(&element_affine, *fq_high)?;

                record_field_elements.push(record_field_element);
            }

            // Fetch the x and y coordinates of the serialized group elements
            // These values will be used in the inner circuit to validate the Elligator2 encoding
            let x = <<C as BaseDPCComponents>::EncryptionModelParameters as ModelParameters>::BaseField::read(
                &to_bytes![element_affine.to_x_coordinate()]?[..],
            )?;
            let y = <<C as BaseDPCComponents>::EncryptionModelParameters as ModelParameters>::BaseField::read(
                &to_bytes![element_affine.to_y_coordinate()]?[..],
            )?;
            record_group_encoding.push((x, y));

            // Construct the plaintext element from the serialized group elements
            // This value will be used in the inner circuit to validate the encryption
            let plaintext_element =
                <<C as DPCComponents>::AccountEncryption as EncryptionScheme>::Text::read(&to_bytes![element]?[..])?;
            record_plaintexts.push(plaintext_element);
        }

        // Encrypt the record plaintext
        let record_public_key = record.account_address().into_repr();
        let encryption_blinding_exponents = circuit_parameters.account_encryption.generate_blinding_exponents(
            record_public_key,
            encryption_randomness,
            record_plaintexts.len(),
        )?;

        Ok((
            record_field_elements,
            record_group_encoding,
            fq_high_bits,
            encryption_blinding_exponents,
        ))
    }
}
