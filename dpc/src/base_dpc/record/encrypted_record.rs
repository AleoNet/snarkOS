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

use crate::base_dpc::BaseDPCComponents;
use snarkos_models::{
    algorithms::EncryptionScheme,
    curves::{AffineCurve, ProjectiveCurve},
    dpc::DPCComponents,
};
use snarkos_utilities::{bits_to_bytes, bytes_to_bits, to_bytes, variable_length_integer::*, FromBytes, ToBytes};

use itertools::Itertools;

use std::io::{Error, ErrorKind, Read, Result as IoResult, Write};

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: BaseDPCComponents"),
    PartialEq(bound = "C: BaseDPCComponents"),
    Eq(bound = "C: BaseDPCComponents")
)]
pub struct EncryptedRecord<C: BaseDPCComponents> {
    pub encrypted_record: Vec<<<C as DPCComponents>::AccountEncryption as EncryptionScheme>::Text>,
    pub final_fq_high_selector: bool,
}

impl<C: BaseDPCComponents> ToBytes for EncryptedRecord<C> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        let mut ciphertext_selectors = Vec::with_capacity(self.encrypted_record.len() + 1);

        // Write the encrypted record
        variable_length_integer(self.encrypted_record.len() as u64).write(&mut writer)?;
        for ciphertext_element in &self.encrypted_record {
            // Compress the ciphertext representation to the affine x-coordinate and the selector bit
            let ciphertext_element_affine =
                <C as BaseDPCComponents>::EncryptionGroup::read(&to_bytes![ciphertext_element]?[..])?.into_affine();

            let x_coordinate = ciphertext_element_affine.to_x_coordinate();
            x_coordinate.write(&mut writer)?;

            let selector =
                match <<C as BaseDPCComponents>::EncryptionGroup as ProjectiveCurve>::Affine::from_x_coordinate(
                    x_coordinate,
                    true,
                ) {
                    Some(affine) => ciphertext_element_affine == affine,
                    None => false,
                };

            ciphertext_selectors.push(selector);
        }

        ciphertext_selectors.push(self.final_fq_high_selector);

        // Write the ciphertext and fq_high selector bits
        let selector_bytes = bits_to_bytes(&ciphertext_selectors);
        selector_bytes.write(&mut writer)?;

        Ok(())
    }
}

impl<C: BaseDPCComponents> FromBytes for EncryptedRecord<C> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the ciphertext x coordinates
        let num_ciphertext_elements = read_variable_length_integer(&mut reader)?;
        let mut ciphertext_x_coordinates = Vec::with_capacity(num_ciphertext_elements);
        for _ in 0..num_ciphertext_elements {
            let ciphertext_element_x_coordinate: <<<C as BaseDPCComponents>::EncryptionGroup as ProjectiveCurve>::Affine as AffineCurve>::BaseField =
                FromBytes::read(&mut reader)?;
            ciphertext_x_coordinates.push(ciphertext_element_x_coordinate);
        }

        // Read the selector bits

        let num_selector_bytes = num_ciphertext_elements / 8 + 1;
        let mut selector_bytes = vec![0u8; num_selector_bytes];
        reader.read_exact(&mut selector_bytes)?;

        let selector_bits = bytes_to_bits(&selector_bytes);
        let ciphertext_selectors = &selector_bits[0..num_ciphertext_elements];
        let final_fq_high_selector = selector_bits[num_ciphertext_elements];

        // Recover the ciphertext
        let mut ciphertext = Vec::with_capacity(ciphertext_x_coordinates.len());
        for (x_coordinate, ciphertext_selector_bit) in ciphertext_x_coordinates.iter().zip_eq(ciphertext_selectors) {
            let ciphertext_element_affine =
                match <<C as BaseDPCComponents>::EncryptionGroup as ProjectiveCurve>::Affine::from_x_coordinate(
                    *x_coordinate,
                    *ciphertext_selector_bit,
                ) {
                    Some(affine) => affine,
                    None => return Err(Error::new(ErrorKind::Other, "Could not read ciphertext")),
                };

            let ciphertext_element: <C::AccountEncryption as EncryptionScheme>::Text =
                FromBytes::read(&to_bytes![ciphertext_element_affine.into_projective()]?[..])?;

            ciphertext.push(ciphertext_element);
        }

        Ok(Self {
            encrypted_record: ciphertext,
            final_fq_high_selector,
        })
    }
}
