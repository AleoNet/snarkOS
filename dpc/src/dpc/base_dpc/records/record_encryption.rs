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

pub fn prepare_encryption_gadget_components<C: BaseDPCComponents>(
    circuit_parameters: &CircuitParameters<C>,
    records: &Vec<DPCRecord<C>>,
    encryption_randomness: &Vec<<<C as DPCComponents>::AccountEncryption as EncryptionScheme>::Randomness>,
) -> Result<
    (
        Vec<Vec<<C::EncryptionModelParameters as ModelParameters>::BaseField>>,
        Vec<
            Vec<(
                <C::EncryptionModelParameters as ModelParameters>::BaseField,
                <C::EncryptionModelParameters as ModelParameters>::BaseField,
            )>,
        >,
        Vec<Vec<bool>>,
        Vec<Vec<<C::AccountEncryption as EncryptionScheme>::BlindingExponent>>,
    ),
    DPCError,
> {
    assert_eq!(records.len(), C::NUM_OUTPUT_RECORDS);
    assert_eq!(encryption_randomness.len(), C::NUM_OUTPUT_RECORDS);

    let mut new_records_field_elements = Vec::with_capacity(C::NUM_OUTPUT_RECORDS);
    let mut new_records_group_encoding = Vec::with_capacity(C::NUM_OUTPUT_RECORDS);
    let mut fq_high_selectors = Vec::with_capacity(C::NUM_OUTPUT_RECORDS);
    let mut new_records_encryption_blinding_exponents = Vec::with_capacity(C::NUM_OUTPUT_RECORDS);
    for (record, encryption_rand) in records.iter().zip_eq(encryption_randomness) {
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
            encryption_rand,
            record_plaintexts.len(),
        )?;

        // Store the field elements and group encodings for each new record
        new_records_field_elements.push(record_field_elements);
        new_records_group_encoding.push(record_group_encoding);
        fq_high_selectors.push(fq_high_bits);
        new_records_encryption_blinding_exponents.push(encryption_blinding_exponents);
    }

    Ok((
        new_records_field_elements,
        new_records_group_encoding,
        fq_high_selectors,
        new_records_encryption_blinding_exponents,
    ))
}

pub fn record_ciphertext_hash<C: BaseDPCComponents>(
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

        let greatest = match <<C as BaseDPCComponents>::EncryptionGroup as ProjectiveCurve>::Affine::from_x_coordinate(
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

pub fn record_ciphertext_hashes<C: BaseDPCComponents>(
    circuit_parameters: &CircuitParameters<C>,
    record_ciphertexts: &Vec<Vec<<<C as DPCComponents>::AccountEncryption as EncryptionScheme>::Text>>,
    final_fq_high_selectors: Vec<bool>,
) -> Result<Vec<<<C as DPCComponents>::RecordCiphertextCRH as CRH>::Output>, DPCError> {
    assert_eq!(record_ciphertexts.len(), C::NUM_OUTPUT_RECORDS);
    assert_eq!(final_fq_high_selectors.len(), C::NUM_OUTPUT_RECORDS);

    let mut ciphertext_hashes = vec![];

    for (record_ciphertext, final_fq_high_selector) in record_ciphertexts.iter().zip_eq(final_fq_high_selectors) {
        let ciphertext_hash = record_ciphertext_hash(circuit_parameters, record_ciphertext, final_fq_high_selector)?;
        ciphertext_hashes.push(ciphertext_hash);
    }

    Ok(ciphertext_hashes)
}

pub fn encrypt_records<C: BaseDPCComponents, R: Rng>(
    circuit_parameters: &CircuitParameters<C>,
    records: &Vec<DPCRecord<C>>,
    rng: &mut R,
) -> Result<
    (
        Vec<<<C as DPCComponents>::AccountEncryption as EncryptionScheme>::Randomness>,
        Vec<Vec<<<C as DPCComponents>::AccountEncryption as EncryptionScheme>::Text>>,
        Vec<Vec<bool>>,
        Vec<bool>,
    ),
    DPCError,
> {
    let mut new_records_encryption_randomness = Vec::with_capacity(C::NUM_OUTPUT_RECORDS);
    let mut new_records_encryption_ciphertexts = Vec::with_capacity(C::NUM_OUTPUT_RECORDS);
    let mut new_records_ciphertext_selectors = Vec::with_capacity(C::NUM_OUTPUT_RECORDS);
    let mut new_records_final_fq_high_selector = Vec::with_capacity(C::NUM_OUTPUT_RECORDS);
    for record in records {
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

        new_records_encryption_randomness.push(encryption_randomness);
        new_records_encryption_ciphertexts.push(record_ciphertext);
        new_records_ciphertext_selectors.push(ciphertext_selectors);
        new_records_final_fq_high_selector.push(final_fq_high_bit);
    }

    Ok((
        new_records_encryption_randomness,
        new_records_encryption_ciphertexts,
        new_records_ciphertext_selectors,
        new_records_final_fq_high_selector,
    ))
}
